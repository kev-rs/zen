use crate::io_cpu_tasks::thread_pool::ThreadPool;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::channel;
use std::{cmp::Ordering, ffi::OsStr, fs, thread};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct FileRecord {
    name: String,
    path: String,
    is_directory: bool,
    icon: String,
    ext: String,
}

impl PartialEq for FileRecord {
    fn eq(&self, other: &Self) -> bool {
        self.is_directory == other.is_directory
    }
}

impl Eq for FileRecord {}

impl PartialOrd for FileRecord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileRecord {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.is_directory && !other.is_directory {
            Ordering::Less
        } else if !self.is_directory && other.is_directory {
            Ordering::Greater
        } else {
            self.name.cmp(&other.name)
        }
    }
}

fn open_dir(dir: &str) -> String {
    let mut result: Vec<FileRecord> = Vec::new();
    let entries = fs::read_dir(dir).unwrap();

    for entry in entries {
        if let Ok(entry) = entry {
            let (name, ext, entry_path, is_dir) = parse_entry(&entry);

            let file = FileRecord {
                name: name.clone(),
                path: entry_path,
                is_directory: is_dir,
                icon: get_icon_for_file(&name, is_dir).to_string(), // A function to determine the icon
                ext,
            };
            result.push(file)
        }
    }
    sort(&mut result);
    serde_json::to_string(&result).unwrap()
}

fn parse_entry(entry: &fs::DirEntry) -> (String, String, String, bool) {
    let entry_path = entry.path();
    let is_dir = entry.file_type().unwrap().is_dir();
    let file_name = entry_path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let ext: String = {
        let ext = entry_path.extension();
        if ext.is_none() && entry_path.is_file() {
            file_name.clone()
        } else {
            ext.unwrap_or(OsStr::new("dir"))
                .to_os_string()
                .into_string()
                .unwrap()
        }
    };

    let path = entry_path.display().to_string();

    (file_name, ext, path, is_dir)
}

pub fn search2(input: &str, path: &str) -> String {
    let pool = ThreadPool::new(4);
    let entries: Vec<_> = fs::read_dir(path).unwrap().collect();
    let mut result = Vec::new();
    let (tx, rx) = channel::<Vec<FileRecord>>();

    for entry in entries {
        if let Ok(entry) = entry {
            let (file_name, ext, fs_path, is_dir) = parse_entry(&entry);

            if is_dir {
                let input = input.to_owned();
                let path = path.to_owned();
                let file_name = file_name.clone();
                let tx = tx.clone();

                pool.execute(move || {
                    let sub_path = file_name.clone();
                    let sub_result = search(&input, &format!("{}/{}", path, sub_path));
                    let sub_result: Vec<FileRecord> = serde_json::from_str(&sub_result)?;
                    tx.send(sub_result)?;
                    Ok(())
                })
                .unwrap();
            }

            if file_name.contains(input) {
                let file = FileRecord {
                    name: file_name.clone(),
                    path: fs_path,
                    is_directory: is_dir,
                    icon: get_icon_for_file(&file_name, is_dir).to_string(),
                    ext,
                };
                result.push(file);
            }
        }
    }

    // Collect results from all threads
    for received in rx.iter() {
        result.extend(received);
    }

    sort(&mut result);
    serde_json::to_string(&result).unwrap()
}
pub fn search(input: &str, path: &str) -> String {
    let entries: Vec<_> = fs::read_dir(path).unwrap().collect();

    let mut result: Vec<FileRecord> = entries
        .par_iter()
        .filter_map(|entry| {
            if let Ok(entry) = entry {
                let (file_name, ext, fs_path, is_dir) = parse_entry(entry);

                let mut files = Vec::new();

                if is_dir {
                    let sub_path = file_name.clone();
                    let sub_result = search(&input, &format!("{}/{}", path, sub_path));
                    let sub_result: Vec<FileRecord> = serde_json::from_str(&sub_result).unwrap();
                    files.extend(sub_result);
                }

                if file_name.contains(input) {
                    let file = FileRecord {
                        name: file_name.clone(),
                        path: fs_path,
                        is_directory: is_dir,
                        icon: get_icon_for_file(&file_name, is_dir).to_string(),
                        ext,
                    };
                    files.push(file);
                }

                Some(files)
            } else {
                None
            }
        })
        .flatten()
        .collect();

    sort(&mut result);
    serde_json::to_string(&result).unwrap()
}

fn sort(arr: &mut [FileRecord]) {
    let len = arr.len();
    if len < 2 {
        return;
    }
    let mid = len / 2;
    let (left, right) = arr.split_at_mut(mid);
    let left = left.to_vec();
    let right = right.to_vec();

    let handle = thread::spawn(move || {
        let mut left = left.clone();
        sort(&mut left);
        left
    });

    let mut right = right.clone();
    sort(&mut right);

    let mut left = handle.join().unwrap();

    merge(arr, &mut left, &mut right);
}

fn merge(arr: &mut [FileRecord], left: &mut [FileRecord], right: &mut [FileRecord]) {
    let (mut i, mut j, mut k) = (0, 0, 0);

    while i < left.len() && j < right.len() {
        match left[i].cmp(&right[j]) {
            Ordering::Less => {
                arr[k] = left[i].clone();
                i += 1;
            }
            Ordering::Greater => {
                arr[k] = right[j].clone();
                j += 1;
            }
            Ordering::Equal => {
                arr[k] = left[i].clone();
                i += 1;
            }
        }
        k += 1;
    }

    if i < left.len() {
        arr[k..].clone_from_slice(&left[i..]);
    }
    if j < right.len() {
        arr[k..].clone_from_slice(&right[j..]);
    }
}

/*
#[tauri::command]
fn search2(input: &str, path: &str) -> String {
    let mut result: Vec<FileRecord> = Vec::new();
    let (tx, rx) = channel::<Vec<FileRecord>>();
    let entries = fs::read_dir(path).unwrap();
    let mut threads = vec![];

    for entry in entries {
        if let Ok(entry) = entry {
            let entry_path = entry.path();
            let is_dir = entry.file_type().unwrap().is_dir();
            let file_name = entry_path.file_stem().unwrap().to_str().unwrap().to_string();

            if is_dir {
                let input = input.to_owned();
                let path = path.to_owned();
                let file_name = file_name.clone();
                let tx = tx.clone();

                let thread_handle = thread::spawn(move || {
                    let sub_path = file_name.clone();
                    let sub_result = search(&input.clone(), &format!("{}/{sub_path}", path.clone()));
                    let mut sub_result = serde_json::from_str(&sub_result).unwrap();
                    tx.send(sub_result).unwrap();
                });
                threads.push(thread_handle);
            }

            let name = file_name;
            let ext = {
                let ext = entry_path.extension();
                if ext.is_none() && entry_path.is_file() {
                    name.clone()
                } else {
                    ext.unwrap_or(OsStr::new("dir")).to_os_string().into_string().unwrap()
                }
            };

            if name.contains(input) {
                let file = FileRecord {
                    name: name.clone(),
                    path: entry_path.display().to_string(),
                    is_directory: is_dir,
                    icon: get_icon_for_file(&name, is_dir).to_string(), // A function to determine the icon
                    ext,
                };
                result.push(file);
            }
        }
    }
    // Join all threads
    for thread in threads {
        thread.join().unwrap();
    }

    // Collect results from all threads
    for received in rx.try_iter() {
        result.extend(received);
    }

    result.sort();
    serde_json::to_string(&result).unwrap()
}
*/
/*
#[tauri::command]
fn search3(input: &str, path: &str) -> String {
    let mut result: Vec<FileRecord> = Vec::new();
    let (tx, rx) = channel::<Vec<FileRecord>>();
    let entries = fs::read_dir(path).unwrap();
    let mut threads = vec![];

    for entry in entries {
        let thread_handle = thread::spawn(move || {
            if let Ok(entry) = entry {
                let entry_path = entry.path();
                let is_dir = entry.file_type().unwrap().is_dir();
                let file_name = entry_path.file_stem().unwrap().to_str().unwrap().to_string();

                if is_dir {
                    let input = input.to_owned();
                    let path = path.to_owned();
                    let file_name = file_name.clone();
                    let tx = tx.clone();

                    let sub_path = file_name.clone();
                    let sub_result = search(&input.clone(), &format!("{}/{sub_path}", path.clone()));
                    let mut sub_result = serde_json::from_str(&sub_result).unwrap();
                    tx.send(sub_result).unwrap();
                }

                let name = file_name;
                let ext = {
                    let ext = entry_path.extension();
                    if ext.is_none() && entry_path.is_file() {
                        name.clone()
                    } else {
                        ext.unwrap_or(OsStr::new("dir")).to_os_string().into_string().unwrap()
                    }
                };

                if name.contains(input) {
                    let file = FileRecord {
                        name: name.clone(),
                        path: entry_path.display().to_string(),
                        is_directory: is_dir,
                        icon: get_icon_for_file(&name, is_dir).to_string(), // A function to determine the icon
                        ext,
                    };
                    result.push(file);
                }
            }
        });
        threads.push(thread_handle);
    }
    // Join all threads
    for thread in threads {
        thread.join().unwrap();
    }

    // Collect results from all threads
    for received in rx.try_iter() {
        result.extend(received);
    }

    result.sort();
    serde_json::to_string(&result).unwrap()
}
*/

#[macro_export]
macro_rules! map {
    ($( $key:expr => $val:expr ),*) => {{
        let mut map = ::std::collections::HashMap::new();
        $( map.insert($key, $val); )*
        map
    }}
}

fn get_icon_for_file(file_name: &str, is_dir: bool) -> String {
    let icon_map = map! {
        "txt" => "/src-tauri/icons/icons8-file-24.png",
        "jpg" => "/src-tauri/icons/icons8-file-24.png",
        "pdf" => "/src-tauri/icons/icons8-file-24.png",
        "dir" => "/src-tauri/icons/icons8-folder-24.png"
    };

    if is_dir {
        return icon_map.get(&"dir").unwrap().to_string();
    }
    let ext = file_name.split('.').last().unwrap_or("dir");
    icon_map
        .get(&ext)
        .unwrap_or(&"/src-tauri/icons/icons8-file-24.png")
        .to_string()
}
