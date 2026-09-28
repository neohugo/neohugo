//! Unit tests of the build-wide state of `deps.Deps` (the construction path, `Deps::init` and
//! `clone_for`, is tested by nh-hugolib's `construction` test on a real site).

use std::sync::{Arc, Mutex};

use nh_common::herrors::Error;
use nh_common::identity::Incrementer;
use nh_common::loggers::{Level, LogSink, Logger, Options};
use nh_deps::deps::{BuildState, GlobalErrHandler, Listeners};

#[test]
fn build_state() {
    let bs = BuildState::default();
    assert_eq!(bs.incr(), 1);
    assert_eq!(bs.incr(), 2);
    bs.add_filename_with_post_prefix("/b.html");
    bs.add_filename_with_post_prefix("/a.html");
    bs.add_filename_with_post_prefix("/b.html");
    // Go: GetFilenamesWithPostPrefix sorts.
    assert_eq!(bs.get_filenames_with_post_prefix(), ["/a.html", "/b.html"]);
    bs.add_filename_with_deferred_prefix("/d.html");
    assert_eq!(bs.get_filenames_with_deferred_prefix(), ["/d.html"]);
}

#[test]
fn listeners() {
    let l: Listeners<i32> = Listeners::default();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let s1 = seen.clone();
    l.add(Box::new(move |vs| {
        s1.lock().unwrap().push(("keep", vs.to_vec()));
        false
    }));
    let s2 = seen.clone();
    l.add(Box::new(move |vs| {
        s2.lock().unwrap().push(("once", vs.to_vec()));
        true
    }));
    l.notify(&[1]);
    l.notify(&[2, 3]);
    assert_eq!(
        *seen.lock().unwrap(),
        [("keep", vec![1]), ("once", vec![1]), ("keep", vec![2, 3])]
    );
}

#[test]
fn global_err_handler() {
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink: LogSink = buf.clone();
    let log = Logger::with_options(Options {
        level: Level::Warn,
        std_out: Some(sink.clone()),
        std_err: Some(sink),
        ..Default::default()
    });
    let h = GlobalErrHandler::new(log.clone());
    // No collector: logged as an error.
    h.send_error(Error::new("first"));
    assert_eq!(log.log_counter_errors(), 1);
    h.start_error_collector();
    for i in 0..60 {
        h.send_error(Error::new(format!("e{i}")));
    }
    let errs = h.stop_error_collector();
    assert_eq!(errs.len(), 50);
    assert_eq!(errs[0].message(), "e0");
    assert_eq!(log.log_counter_errors(), 1);
}
