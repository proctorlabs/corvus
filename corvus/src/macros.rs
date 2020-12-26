#[macro_export]
macro_rules! service_interval {
    (($($time:tt)*) : { $($content:tt)* } ) => {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::new($( $time )*, 0));
            interval.tick().await;
            loop {
                interval.tick().await;
                let r = {
                    $( $content )*
                    Ok::<(), anyhow::Error>(())
                };
                match r {
                    Ok(_) => continue,
                    Err(e) => error!("Task failure! {:?}", e),
                }
            }
            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        })
    };
}

#[macro_export]
macro_rules! spawn {
    ($($content:tt)*) => {
        tokio::spawn(async move {
            // use tokio_stream::StreamExt;
            let mut throttle = tokio::time::interval(std::time::Duration::from_secs(2));
            loop {
                throttle.tick().await;
                #[allow(unused_variables)]
                let r = async {
                    $( $content )*
                    #[allow(unreachable_code)]
                    Ok::<(), anyhow::Error>(())
                }.await;
                match r {
                    Ok(_) => warn!("Task prematurely finished!"),
                    Err(e) => error!("Task failed: {:?}", e),
                }
            }
            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        })
    };
}
