/// Very simplistic implementation for our usage, I didn't find any generic impl's on crates.io
#[derive(Debug, Default, Clone)]
pub struct RingBuffer<T> {
    pos:  usize,
    size: usize,
    data: [T; 6],
}

impl<T> RingBuffer<T> {
    const SIZE: usize = 6;

    pub fn sample(&self) -> T
    where
        T: Clone,
    {
        self.data[self.pos].clone()
    }

    pub fn get_recent(&self, mut amt: usize) -> Vec<T>
    where
        T: Clone,
    {
        let mut res = vec![];
        if amt > Self::SIZE {
            amt = Self::SIZE;
        }

        for i in 0..amt {
            let mut pos: isize = (self.pos as isize) - (i as isize);
            if pos < 0 {
                pos += Self::SIZE as isize;
            }
            res.push(self.data[pos as usize].clone())
        }
        res
    }

    pub fn push(&mut self, o: T) {
        self.pos = (self.pos + 1) % Self::SIZE;
        if self.size < Self::SIZE {
            self.size += 1;
        }
        self.data[self.pos] = o;
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn push() {
        let mut rb = RingBuffer::<usize>::default();
        rb.push(5);
        assert_eq!(5, rb.sample());
    }

    #[test]
    fn push_three() {
        let mut rb = RingBuffer::<usize>::default();
        rb.push(5);
        rb.push(2);
        rb.push(10);
        assert_eq!(10, rb.sample());
        assert_eq!(vec![10, 2, 5], rb.get_recent(3));
    }

    #[test]
    fn push_ten() {
        let mut rb = RingBuffer::<usize>::default();
        assert_eq!(0, rb.size());
        rb.push(1);
        rb.push(2);
        rb.push(3);
        assert_eq!(3, rb.size());
        rb.push(4);
        rb.push(5);
        rb.push(6);
        assert_eq!(6, rb.size());
        rb.push(7);
        rb.push(8);
        rb.push(9);
        rb.push(10);
        assert_eq!(6, rb.size());
        assert_eq!(10, rb.sample());
        assert_eq!(vec![10, 9, 8], rb.get_recent(3));
        assert_eq!(vec![10, 9, 8, 7, 6, 5], rb.get_recent(10));
    }
}
