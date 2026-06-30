use crossbeam_channel::unbounded;
use jarvis_lib::domain::voice::VoiceState;
use jarvis_lib::handlers::voice::get_status;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

fn make_test_state() -> VoiceState {
    let (tx, _rx) = unbounded();
    VoiceState {
        command_tx: tx,
        is_transcribing: Arc::new(AtomicBool::new(false)),
        latest_transcript: Arc::new(Mutex::new(String::new())),
        completion_notifier: Arc::new((Mutex::new(false), Condvar::new())),
    }
}

// ── compare_exchange: starting when ALREADY transcribing returns Err ─────

#[tokio::test]
async fn start_when_already_transcribing_fails() {
    let state = make_test_state();
    let timeout = Duration::from_millis(500);

    let result = tokio::time::timeout(timeout, async {
        // Simulate already-transcribing state
        state.is_transcribing.store(true, Ordering::SeqCst);

        // compare_exchange(false, true) must fail because current is true
        let cas =
            state
                .is_transcribing
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);
        cas.is_err()
    })
    .await
    .expect("timeout");

    assert!(
        result,
        "compare_exchange should fail when already transcribing"
    );
    assert!(get_status(&state));
}

// ── compare_exchange: starting when NOT transcribing succeeds ────────────

#[tokio::test]
async fn start_when_not_transcribing_succeeds() {
    let state = make_test_state();
    let timeout = Duration::from_millis(500);

    let result = tokio::time::timeout(timeout, async {
        assert!(!state.is_transcribing.load(Ordering::SeqCst));

        let cas =
            state
                .is_transcribing
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);
        cas.is_ok()
    })
    .await
    .expect("timeout");

    assert!(
        result,
        "compare_exchange should succeed when not transcribing"
    );
    assert!(get_status(&state));
}

// ── compare_exchange: stopping when NOT transcribing (already false) ─────

#[tokio::test]
async fn stop_when_not_transcribing_is_noop() {
    let state = make_test_state();
    let timeout = Duration::from_millis(500);

    let result = tokio::time::timeout(timeout, async {
        // is_transcribing is already false
        assert!(!state.is_transcribing.load(Ordering::SeqCst));

        // Simulating a stop: compare_exchange(true, false) should fail
        let cas =
            state
                .is_transcribing
                .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst);
        cas.is_err()
    })
    .await
    .expect("timeout");

    assert!(
        result,
        "compare_exchange(true→false) should fail when already false"
    );
    assert!(!get_status(&state));
}

// ── compare_exchange: stopping when transcribing succeeds ────────────────

#[tokio::test]
async fn stop_when_transcribing_succeeds() {
    let state = make_test_state();
    let timeout = Duration::from_millis(500);

    let result = tokio::time::timeout(timeout, async {
        state.is_transcribing.store(true, Ordering::SeqCst);
        assert!(get_status(&state));

        let cas =
            state
                .is_transcribing
                .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst);
        cas.is_ok()
    })
    .await
    .expect("timeout");

    assert!(
        result,
        "compare_exchange(true→false) should succeed when transcribing"
    );
    assert!(!get_status(&state));
}

// ── Condvar completion notification fires after signal ───────────────────

#[tokio::test]
async fn condvar_notification_fires_on_complete() {
    let state = make_test_state();
    let timeout = Duration::from_millis(500);

    // Set a transcript
    {
        let mut t = state.latest_transcript.lock().unwrap();
        *t = "hello world".to_string();
    }

    let notifier = state.completion_notifier.clone();
    let transcript = state.latest_transcript.clone();

    // Spawn a thread that waits on the condvar (like the watcher in start_transcription)
    let handle = std::thread::spawn(move || {
        let (lock, cvar) = &*notifier;
        let mut completed = lock.lock().unwrap();
        while !*completed {
            completed = cvar.wait(completed).unwrap();
        }
        drop(completed);
        let t = transcript.lock().unwrap().clone();
        t
    });

    // Give the watcher thread time to start waiting
    std::thread::sleep(Duration::from_millis(20));

    // Signal completion (simulating what the worker thread does)
    {
        let (lock, cvar) = &*state.completion_notifier;
        let mut completed = lock.lock().unwrap();
        *completed = true;
        cvar.notify_one();
    }

    let result = tokio::time::timeout(timeout, async {
        tokio::task::spawn_blocking(move || handle.join().expect("watcher thread panicked"))
            .await
            .expect("spawn_blocking failed")
    })
    .await
    .expect("timeout: condvar notification did not fire");

    assert_eq!(result, "hello world");
}

// ── Condvar: watcher not notified when flag stays false ──────────────────

#[tokio::test]
async fn condvar_not_fired_when_not_signaled() {
    let state = make_test_state();
    let notifier = state.completion_notifier.clone();

    // Verify the flag is initially false
    {
        let (lock, _) = &*notifier;
        let completed = lock.lock().unwrap();
        assert!(!*completed, "completion flag should start as false");
    }

    // Wait with a short timeout — should time out since we don't signal
    let timed_out = {
        let (lock, cvar) = &*notifier;
        let completed = lock.lock().unwrap();
        let result = cvar
            .wait_timeout(completed, Duration::from_millis(500))
            .unwrap();
        result.1.timed_out() // true if timed out
    };

    assert!(
        timed_out,
        "condvar wait should have timed out without notification"
    );
}
