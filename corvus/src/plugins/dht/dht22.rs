use crate::prelude::*;
use linux_embedded_hal::{gpio_cdev::*, CdevPin};
use nix::unistd::close;
use parking_lot::Mutex;
use std::{
    fmt,
    fmt::Debug,
    os::unix::io::AsRawFd,
    time::{Duration, Instant},
};

#[derive(Debug)]
pub enum Errors {
    Timeout(u8),
    Checksum,
}

pub enum DHTState {
    Init,
    Write(CdevPin),
    Read(CdevPin),
}

impl Debug for DHTState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DHTState").finish()
    }
}

impl Clone for DHTState {
    fn clone(&self) -> Self {
        Self::Init
    }
}

#[derive(Debug, Clone, Deref)]
pub struct DHT(Arc<Mutex<DHTInner>>);

#[derive(Debug, Clone)]
pub struct DHTInner {
    line:      Option<Line>,
    state:     DHTState,
    gpio_path: String,
    pin_num:   u32,
}

#[derive(Copy, Clone, Debug)]
struct Pulse {
    lo: u8,
    hi: u8,
}

#[derive(Debug, Clone)]
pub struct Reading {
    pub humidity:    f32,
    pub temperature: f32,
    pub fahrenheit:  f32,
}

impl std::error::Error for Errors {}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Errors::Timeout(i) => write!(f, "Timeout! {}", i),
            Errors::Checksum => write!(f, "Checksum!"),
        }
    }
}

fn wait_ms(n: u64) {
    // spin_sleep::sleep(Duration::from_millis(n));
    std::thread::sleep(Duration::from_millis(n));
}

fn wait_us(n: u64) {
    spin_sleep::sleep(Duration::from_micros(n));
}

impl DHT {
    pub fn new(gpio_path: &str, pin_num: u32) -> Result<Self> {
        let mut chip = Chip::new(gpio_path)?;
        let line = chip.get_line(pin_num)?;
        Ok(DHT(Arc::new(Mutex::new(DHTInner {
            line: Some(line),
            state: DHTState::Init,
            gpio_path: gpio_path.to_string(),
            pin_num,
        }))))
    }
    pub fn get_reading(&mut self) -> Result<Reading> {
        let mut inner = self.lock();
        inner.get_reading()
    }
}

impl DHTInner {
    fn start_signal(&mut self) -> Result<()> {
        let pin = self.get_writer()?;
        pin.set_value(1)?;
        wait_ms(500);
        pin.set_value(0)?;
        wait_us(2000);
        // pin.set_value(1)?;
        // wait_us(25);
        Ok(())
    }

    fn read_until(&mut self, val: u8, timeout: Duration) -> Result<Duration> {
        let start = Instant::now();
        let pin = self.get_reader()?;
        while pin.get_value()? != val {
            wait_us(1);
            if start.elapsed() > timeout {
                return Err(Errors::Timeout(u8::MAX).into());
            }
        }
        Ok(start.elapsed())
    }

    pub fn get_reading(&mut self) -> Result<Reading> {
        // Setup
        self.start_signal()?;
        let mut timings: Vec<(i64, i64)> = vec![];
        let mut cur_state = self.get_reader()?.get_value()?;
        let mut low_ts = Instant::now();
        let mut high_ts = Instant::now();

        // Reading loop
        let mut last_timing: Option<(i64, i64)> = None;
        loop {
            let next_state = (cur_state + 1) & 1;
            let res = self.read_until(next_state, Duration::from_micros(500));
            if res.is_ok() {
                cur_state = next_state;
                if cur_state == 0 {
                    let dur = low_ts.elapsed();
                    low_ts = Instant::now();
                    let mut new_val = last_timing.take().unwrap_or((0, 0));
                    new_val.0 = (dur.as_micros() as i64) - 50;
                    timings.push(new_val);
                } else {
                    let dur = high_ts.elapsed();
                    high_ts = Instant::now();
                    if let Some(val) = last_timing {
                        timings.push(val);
                    }
                    last_timing = Some((0, (dur.as_micros() as i64) - 50));
                }
            } else {
                break;
            }
        }

        // debug!("Data: ({}) {:?}", timings.len(), timings);
        self.get_writer()?.set_value(1)?;
        // Return parsed result
        if timings.len() < 40 {
            Err(Errors::Timeout(timings.len() as u8).into())
        } else {
            Ok(Reading::from_pulses(timings)?)
        }
    }

    fn get_writer(&mut self) -> Result<&CdevPin> {
        if let DHTState::Write(ref r) = self.state {
            return Ok(r);
        }

        self.reset()?;
        let handle = self.line.as_ref().unwrap().request(
            LineRequestFlags::OUTPUT | LineRequestFlags::OPEN_DRAIN,
            1,
            "DHT-Util",
        )?;
        let pin = CdevPin::new(handle)?;
        self.state = DHTState::Write(pin);
        if let DHTState::Write(ref w) = self.state {
            return Ok(w);
        }
        unreachable!()
    }

    fn get_reader(&mut self) -> Result<&CdevPin> {
        if let DHTState::Read(ref r) = self.state {
            return Ok(r);
        }

        self.reset()?;
        let handle = self.line.as_ref().unwrap().request(
            LineRequestFlags::INPUT | LineRequestFlags::ACTIVE_LOW,
            1,
            "DHT-Util",
        )?;
        let pin = CdevPin::new(handle)?;
        self.state = DHTState::Read(pin);
        if let DHTState::Read(ref r) = self.state {
            return Ok(r);
        }
        unreachable!()
    }

    fn close(&mut self) -> Result<()> {
        match &mut self.state {
            DHTState::Read(ref r) => {
                close(r.as_raw_fd())?;
                Ok(())
            }
            DHTState::Write(ref w) => {
                close(w.as_raw_fd())?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
    fn reset(&mut self) -> Result<()> {
        self.close()?;
        self.state = DHTState::Init;
        Ok(())
    }
}

const CUTOFF: i64 = 100;

impl Reading {
    fn from_pulses(timings: Vec<(i64, i64)>) -> Result<Self> {
        // Convert pulse timings into data
        trace!("Original Values: ({}) {:?}", timings.len(), timings);

        let mut cleaned_values = vec![];
        for (lo, hi) in timings.iter() {
            match (lo, hi) {
                (lo, hi) if lo < &CUTOFF && hi < &CUTOFF => {
                    let val = (lo + hi) / 2;
                    cleaned_values.push(val);
                }
                (lo, hi) if ((lo + hi) / 2) > CUTOFF => {
                    //This will likely fail, but it's a last ditch effort to fix bad values
                    let val = ((lo - 50) + (hi - 50)) / 4;
                    if val > 40 && val < 60 {
                        cleaned_values.push(val + 25);
                        cleaned_values.push(val - 25);
                    } else {
                        cleaned_values.push(val);
                        cleaned_values.push(val);
                    }
                }
                (lo, hi) if lo > &CUTOFF => {
                    cleaned_values.push(*hi);
                    cleaned_values.push(lo - hi - 50);
                }
                (lo, _) => {
                    // hi > cutoff
                    cleaned_values.push(*lo);
                }
            }
        }
        if cleaned_values.len() < 40 {
            return Err(Errors::Timeout(timings.len() as u8).into());
        }
        trace!(
            "Cleaned Values: ({}) {:?}",
            cleaned_values.len(),
            cleaned_values
        );
        // debug!("Normalized {:?}", normalized);
        // let c = timings.len() - 39;
        let set = &cleaned_values[cleaned_values.len() - 40..];
        let mut bytes = [0u8; 5];
        for (i, pulses) in set.chunks(8).enumerate() {
            let byte = &mut bytes[i];
            for t in pulses {
                *byte <<= 1;
                if t > &50 {
                    *byte |= 1;
                }
            }
        }

        // Validate the checksum
        let expected = bytes[4];
        let actual = bytes[0] + bytes[1] + bytes[2] + bytes[3];
        if actual != expected {
            trace!(
                "Failed checksum: Actual[{}] Expected[{}] bytes[{:?}]",
                actual,
                expected,
                bytes
            );
            // debug!("Checksum failed, Next.. {}", i);
            return Err(Errors::Checksum.into());
        }
        trace!(
            "Passed checksum: Actual[{}] Expected[{}] bytes[{:?}]",
            actual,
            expected,
            bytes
        );
        let h_dec = bytes[0] as u16 * 256 + bytes[1] as u16;
        let humidity = h_dec as f32 / 10.0f32;

        let t_dec = (bytes[2] & 0x7f) as u16 * 256 + bytes[3] as u16;
        let mut temperature = t_dec as f32 / 10.0f32;
        if (bytes[2] & 0x80) != 0 {
            temperature *= -1.0f32;
        }

        if humidity < 0.0 || humidity > 100.0 || temperature < 0.0 || temperature > 60.0 {
            // debug!("Values out of range, Next.. {}", i);
            return Err(Errors::Checksum.into());
        }

        Ok(Self {
            humidity,
            temperature,
            fahrenheit: (temperature * 1.8) + 32.0,
        })
    }
}
