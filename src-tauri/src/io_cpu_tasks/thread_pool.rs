use std::any::Any;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

type Job = Box<dyn FnOnce() -> Result<(), Box<dyn Error>> + Send + 'static>;
type WorkerID = usize;
type JobsQueue = Arc<(Mutex<VecDeque<Job>>, Condvar, WorkerID)>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    queues: Arc<Mutex<Vec<JobsQueue>>>,
    job_counts: Arc<Vec<AtomicUsize>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 1);

        let mut queues = Arc::new(Mutex::new(Vec::with_capacity(size)));
        let job_counts = Arc::new((0..size).map(|_| AtomicUsize::new(0)).collect::<Vec<_>>());
        let mut workers = (0..size)
            .map(|id| {
                println!("__Worker {} created__", id + 1);
                let queue = Arc::new((Mutex::new(VecDeque::new()), Condvar::new(), id + 1));
                {
                    let mut queues_lock = queues.lock().expect("Failed to lock queues");
                    queues_lock.push(queue.clone());
                }
                Worker::new(id + 1, queue, queues.clone(), job_counts.clone())
            })
            .collect::<Vec<_>>();

        ThreadPool {
            workers,
            queues,
            job_counts,
        }
    }

    pub fn execute<F>(&self, f: F) -> Result<(), Box<dyn Error>>
    where
        F: FnOnce() -> Result<(), Box<dyn Error>> + Send + 'static,
    {
        let job = Box::new(f);

        let (mutex, cvar, id) = &*self.get_worker();
        {
            let mut queue = mutex.lock().unwrap();
            queue.push_back(job);
        }
        self.job_counts[id - 1].fetch_add(1, Ordering::SeqCst);
        cvar.notify_one();

        Ok(())
    }

    fn get_worker(&self) -> JobsQueue {
        let queues = self.queues.lock().unwrap();
        let (least_busiest_idx, _) = self
            .job_counts
            .iter()
            .enumerate()
            .min_by_key(|(_, count)| count.load(Ordering::SeqCst))
            .unwrap();

        queues[least_busiest_idx].clone()
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(
        id: usize,
        local_queue: JobsQueue,
        all_queues: Arc<Mutex<Vec<JobsQueue>>>,
        job_counts: Arc<Vec<AtomicUsize>>,
    ) -> Worker {
        let builder = thread::Builder::new().name(format!("{}", id));
        // let all_queues = all_queues.clone();

        let thread = builder.spawn(move || loop {
            let (mutex, cvar, id) = &*local_queue;
            let mut queue = mutex.lock().unwrap();

            if let Some(job) = queue.pop_back() {
                println!("Worker {}, working...", id);
                job_counts[id - 1].fetch_sub(1, Ordering::SeqCst);
                job().unwrap();
            } else {
                queue = cvar.wait(queue).unwrap();
                // let all_queues = all_queues.try_lock().unwrap();
                // if let Some(q) = get_worker(&all_queues, &job_counts) {
                //     let (mutex, ..) = &*q;
                //     let mut queue = mutex.try_lock().unwrap();

                //     if let Some(job) = queue.pop_front() {
                //         job().unwrap();
                //     } else {
                //         queue = cvar.wait(queue).unwrap();
                //     }
                // } else {
                //     queue = cvar.wait(queue).unwrap();
                // }
            }
        });

        match thread {
            Ok(thread) => Worker {
                id,
                thread: Some(thread),
            },
            Err(err) => panic!("Error from worker constructor: {}", err.to_string()),
        }
    }
}

fn get_worker(queues: &Vec<JobsQueue>, job_counts: &Vec<AtomicUsize>) -> Option<JobsQueue> {
    if queues.len() < job_counts.len() {
        return None;
    }
    println!("job_counts len: {}", job_counts.len());
    println!("all_queues len: {}", queues.len());
    let (busiest_idx, _) = job_counts
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| count.load(Ordering::SeqCst))
        .unwrap();
    println!("Worker {}", busiest_idx + 1);

    return Some(queues[busiest_idx].clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io_cpu_tasks::test::{search, search2};
    use crate::FileRecord;

    #[test]
    fn read_dir() {
        let mut nums = [1; 10];

        let pool = ThreadPool::new(4);
        for i in 0..40 {
            let idx = i;
            pool.execute(move || {
                println!("hello {} times", idx + 1);
                Ok(())
            })
            .unwrap();
        }

        // let input = "src";
        // let path = ".";
        // let result = search2(input, path);
        // let result: Vec<FileRecord> = serde_json::from_str(&result).unwrap();
        // println!("RESULTS: {:#?}", result);
    }
}
