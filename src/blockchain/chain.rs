use memmap::Mmap;
//use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
//use vec_map::VecMap;

//use blockchain::block::Block;
//use blockchain::error::ParseResult;
//use blockchain::hash::{Hash};
//use blockchain::address::Address;

/*
#[derive(PartialEq, Eq, Copy, Clone)]
struct InitIndexEntry<'a> {
    block: Option<Block<'a>>,
    child_hash: Option<Hash>,
}
*/

pub struct Blockchain {
    pub maps: Vec<Mmap>,
}

impl Blockchain {
    pub unsafe fn read(blocks_dir: &str, max_block: usize) -> Blockchain {
        let mut maps: Vec<Mmap> = Vec::new();
        let mut n: usize = 0;
        let blocks_dir_path = PathBuf::from(blocks_dir);

        loop {
            match File::open(blocks_dir_path.join(format!("blk{:05}.dat", n))) {
                Ok(f) => {
                    if n > max_block {
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

        Blockchain { maps }
    }

    /*
    pub fn walk_slice(
        &self,
        mut slice: &[u8],
        goal_prev_hash: &mut Hash,
        last_block: &mut Option<Block>,
        height: &mut u64,
        skipped: &mut HashMap<Hash, Block>,
        output_items: &mut HashMap<Hash, VecMap<Vec<Address>>>,
    ) -> ParseResult<()> {
        //while slice.len() > 0 {
            if skipped.contains_key(goal_prev_hash) {
                //last_block.unwrap().walk(*height, output_items)?;
                *height += 1;
                while let Some(block) = skipped.remove(goal_prev_hash) {
                    block.walk(*height, output_items)?;
                    *height += 1;
                    *goal_prev_hash = block.header().cur_hash();
                    *last_block = None;
                }
            }

            let block = match Block::read(&mut slice)? {
                Some(block) => block,
                None => {
                    assert_eq!(slice.len(), 0);
                    break;
                }
            };

            if block.header().prev_hash() != goal_prev_hash {
                skipped.insert(*block.header().prev_hash(), block);

                if last_block.is_some()
                    && block.header().prev_hash() == last_block.unwrap().header().prev_hash()
                {
                    let first_orphan = last_block.unwrap();
                    let second_orphan = block;

                    loop {
                        let block = match Block::read(&mut slice)? {
                            Some(block) => block,
                            None => {
                                assert_eq!(slice.len(), 0);
                                break;
                            }
                        };
                        skipped.insert(*block.header().prev_hash(), block);
                        if block.header().prev_hash() == &first_orphan.header().cur_hash() {
                            break;
                        }
                        if block.header().prev_hash() == &second_orphan.header().cur_hash() {
                            *goal_prev_hash = second_orphan.header().cur_hash();
                            *last_block = Some(second_orphan);
                            break;
                        }
                    }
                }
                continue;
            }

            if let Some(last_block) = *last_block {
                //last_block.walk(*height, output_items)?;
                *height += 1;
            }

            *goal_prev_hash = block.header().cur_hash();
            *last_block = Some(block);
        //}

        Ok(())
    }
    */
}
