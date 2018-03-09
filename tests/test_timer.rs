extern crate futures;
extern crate tokio_timer as timer;

mod support;

// use futures::*;
use futures::prelude::*;
use futures::channel::{oneshot, mpsc};
use futures::executor::block_on;
use timer::*;
use std::io;
use std::time::*;
use std::thread;

#[test]
fn test_immediate_sleep() {
    let timer = Timer::default();

    let mut t = timer.sleep(Duration::from_millis(0));
    let cx = unsafe{ std::mem::transmute(&mut ()) };
    assert_eq!(Async::Ready(()), t.poll(cx).unwrap());
}

#[test]
fn test_delayed_sleep() {
    let timer = Timer::default();
    let dur = Duration::from_millis(200);

    for _ in 0..20 {
        let elapsed = support::time(|| {
            block_on(timer.sleep(dur)).unwrap();
        });

        elapsed.assert_is_about(dur);
    }
}

#[test]
fn test_setting_later_sleep_then_earlier_one() {
    let timer = Timer::default();

    let dur1 = Duration::from_millis(500);
    let dur2 = Duration::from_millis(200);

    let to1 = timer.sleep(dur1);
    let to2 = timer.sleep(dur2);

    let t1 = thread::spawn(move || {
        support::time(|| block_on(to1).unwrap())
    });

    let t2 = thread::spawn(move || {
        support::time(|| block_on(to2).unwrap())
    });

    t1.join().unwrap().assert_is_about(dur1);
    t2.join().unwrap().assert_is_about(dur2);
}

#[test]
fn test_timer_with_looping_wheel() {
    let timer = timer::wheel()
        .num_slots(8)
        .max_timeout(Duration::from_millis(10_000))
        .build();

    let dur1 = Duration::from_millis(200);
    let dur2 = Duration::from_millis(1000);

    let to1 = timer.sleep(dur1);
    let to2 = timer.sleep(dur2);

    let e1 = support::time(|| block_on(to1).unwrap());
    let e2 = support::time(|| block_on(to2).unwrap());

    e1.assert_is_about(dur1);
    e2.assert_is_about(Duration::from_millis(800));
}

#[test]
fn test_request_sleep_greater_than_max() {
    let timer = timer::wheel()
        .max_timeout(Duration::from_millis(500))
        .build();

    let to = timer.sleep(Duration::from_millis(600));
    assert!(block_on(to).is_err());

    let to = timer.sleep(Duration::from_millis(500));
    assert!(block_on(to).is_ok());
}

#[test]
fn test_timeout_with_future_completes_first() {
    let timer = Timer::default();
    let dur = Duration::from_millis(300);

    let (tx, rx) = oneshot::channel();
    let rx = rx.then(|res| {
        match res {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(e),
            _ => panic!("invalid"),
        }
    });

    let to = timer.timeout(rx, dur);

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        tx.send(Ok::<&'static str, io::Error>("done")).expect("send");
    });

    assert_eq!("done", block_on(to).unwrap());
}

#[test]
fn test_timeout_with_timeout_completes_first() {
    let timer = Timer::default();
    let dur = Duration::from_millis(300);

    let (_tx, rx) = oneshot::channel::<Result<&'static str, io::Error>>();
    let rx = rx.then(|res| {
        match res {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(e),
            _ => panic!("invalid"),
        }
    });

    let to = timer.timeout(rx, dur);

    let err: io::Error = block_on(to).unwrap_err();
    assert_eq!(io::ErrorKind::TimedOut, err.kind());
}

#[test]
fn test_timeout_with_future_errors_first() {
    let timer = Timer::default();
    let dur = Duration::from_millis(300);

    let (tx, rx) = oneshot::channel();
    let rx = rx.then(|res| {
        match res {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(e),
            _ => panic!("invalid"),
        }
    });

    let to = timer.timeout(rx, dur);
    let err = io::Error::new(io::ErrorKind::NotFound, "not found");

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        tx.send(Err::<&'static str, io::Error>(err)).expect("send");
    });

    let err = block_on(to).unwrap_err();

    assert_eq!(io::ErrorKind::NotFound, err.kind());
}

#[test]
fn test_timeout_stream_with_stream_completes_first() {
    let timer = Timer::default();
    let dur = Duration::from_millis(300);

    let (tx, rx) = mpsc::unbounded();
    let rx = rx.then(|res| {
        match res {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(e),
            _ => panic!("invalid"),
        }
    });

    let to = timer.timeout_stream(rx, dur);

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        let f = tx.send(Ok::<&'static str, io::Error>("one"));
        let tx = block_on(f).unwrap();

        thread::sleep(Duration::from_millis(100));
        let f = tx.send(Ok::<&'static str, io::Error>("two"));
        block_on(f).unwrap();
    });

    let mut s = block_on(to);

    assert_eq!("one", s.next().unwrap().unwrap());
    assert_eq!("two", s.next().unwrap().unwrap());
    assert!(s.next().is_none());
}

#[test]
fn test_timeout_stream_with_timeout_completes_first() {
    let timer = Timer::default();
    let dur = Duration::from_millis(300);

    let (tx, rx) = mpsc::unbounded();
    let rx = rx.then(|res| {
        match res {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(e),
            _ => panic!("invalid"),
        }
    });

    let to = timer.timeout_stream(rx, dur);

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        let f = tx.send(Ok::<&'static str, io::Error>("one"));
        let tx = block_on(f).unwrap();

        thread::sleep(Duration::from_millis(100));
        let f = tx.send(Ok::<&'static str, io::Error>("two"));
        let tx = block_on(f).unwrap();

        thread::sleep(Duration::from_millis(600));

        drop(tx);
    });

    let mut s = block_on(to);

    assert_eq!("one", s.next().unwrap().unwrap());
    assert_eq!("two", s.next().unwrap().unwrap());

    let err = s.next().unwrap().unwrap_err();
    assert_eq!(io::ErrorKind::TimedOut, err.kind());
}

#[test]
fn test_interval_once() {
    let timer = Timer::default();
    let dur = Duration::from_millis(300);
    let mut interval = timer.interval(dur).wait();

    let e1 = support::time(|| {
        interval.next();
    });

    e1.assert_is_about(dur);
}

#[test]
fn test_interval_twice() {
    let timer = Timer::default();
    let dur = Duration::from_millis(300);
    let mut interval = timer.interval(dur).wait();

    let e1 = support::time(|| {
        interval.next();
        interval.next();
    });

    e1.assert_is_about(dur * 2);
}

#[test]
fn test_interval_at_once() {
    let timer = Timer::default();
    let delay = Duration::from_millis(500);
    let dur = Duration::from_millis(300);
    let mut interval = timer.interval_at(Instant::now() + delay, dur).wait();

    let e1 = support::time(|| {
        interval.next();
    });

    e1.assert_is_about(delay);
}

#[test]
fn test_interval_at_twice() {
    let timer = Timer::default();
    let delay = Duration::from_millis(500);
    let dur = Duration::from_millis(300);
    let mut interval = timer.interval_at(Instant::now() + delay, dur).wait();

    let e1 = support::time(|| {
        interval.next();
        interval.next();
    });

    e1.assert_is_about(delay + dur);
}

#[test]
fn test_interval_at_past() {
    let timer = Timer::default();
    let dur = Duration::from_millis(300);
    let mut interval = timer.interval_at(Instant::now() - Duration::from_millis(200), dur).wait();

    let e1 = support::time(|| {
        interval.next();
    });

    e1.assert_is_about(Duration::from_millis(0));
}
