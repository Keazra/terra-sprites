//! Runs the terra-sim examples' long jobs, such as one seed's world each,
//! several at once (`baseline`, `childhood`, `dieout`, `generations`).

use std::sync::Mutex;

/// Runs `run` on each of `jobs`, taking them in order, on `workers` threads
/// at once, and gives back what each run returned, in the order they
/// finished.
pub fn in_parallel<J: Send, R: Send>(
    jobs: impl IntoIterator<Item = J, IntoIter: Send>,
    workers: u64,
    run: impl Fn(J) -> R + Sync,
) -> Vec<R> {
    let jobs = Mutex::new(jobs.into_iter());
    let done = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    // Taken on a line of its own, so the lock isn't held
                    // while the job runs.
                    let next = jobs.lock().expect("no thread panicked").next();
                    let Some(job) = next else {
                        break;
                    };
                    let result = run(job);
                    done.lock().expect("no thread panicked").push(result);
                }
            });
        }
    });
    done.into_inner().expect("no thread panicked")
}
