use memmap::Mmap;
use std::fs::File;
use std::path::PathBuf;

pub struct Blockchain {
    pub maps: Vec<Mmap>,
}

impl Blockchain {
    pub fn new(blocks_dir: &str, max_block: usize) -> Self {
        let mut maps: Vec<Mmap> = Vec::new();
        let mut n: usize = 0;
        let blocks_dir_path = PathBuf::from(blocks_dir);

        loop {
            unsafe {
                match File::open(blocks_dir_path.join(format!("blk{:05}.dat", n))) {
                    Ok(f) => {
                        if max_block != 0 && n > max_block {
                            break;
                        }
                        n += 1;
                        match Mmap::map(&f) {
                            Ok(m) => {
                                maps.push(m);
                            }
                            Err(_) => {
                                break;
                            }
                        }
                    }
                    Err(_) => {
                        break;
                    }
                };
            }
        }
        Blockchain { maps }
    }
}
