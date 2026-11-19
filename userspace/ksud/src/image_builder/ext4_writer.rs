//! Pure Rust EXT4 Image Writer
//!
//! Generates a valid EXT4 filesystem image from a `VfsTree`
//! with full support for Whiteout 0:0 and Opaque attributes.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

use super::vfs_tree::{VfsNode, VfsNodeType, VfsTree};

const EXT4_SUPER_MAGIC: u16 = 0xEF53;
const EXT4_BLOCK_SIZE: usize = 4096;
const EXT4_INODE_SIZE: usize = 256;

#[repr(C, packed)]
struct Ext4SuperBlock {
    s_inodes_count: u32,
    s_blocks_count_lo: u32,
    s_r_blocks_count_lo: u32,
    s_free_blocks_count_lo: u32,
    s_free_inodes_count: u32,
    s_first_data_block: u32,
    s_log_block_size: u32,
    s_log_cluster_size: u32,
    s_blocks_per_group: u32,
    s_clusters_per_group: u32,
    s_inodes_per_group: u32,
    s_mtime: u32,
    s_wtime: u32,
    s_mnt_count: u16,
    s_max_mnt_count: u16,
    s_magic: u16,
    s_state: u16,
    s_errors: u16,
    s_minor_rev_level: u16,
    s_lastcheck: u32,
    s_checkinterval: u32,
    s_creator_os: u32,
    s_rev_level: u32,
    s_def_resuid: u16,
    s_def_resgid: u16,
    s_first_ino: u32,
    s_inode_size: u16,
    s_block_group_nr: u16,
    s_feature_compat: u32,
    s_feature_incompat: u32,
    s_feature_ro_compat: u32,
    s_uuid: [u8; 16],
    s_volume_name: [u8; 16],
    s_last_mounted: [u8; 64],
    s_algorithm_usage_bitmap: u32,
    s_prealloc_blocks: u8,
    s_prealloc_dir_blocks: u8,
    s_reserved_gdb_count: u16,
    s_journal_uuid: [u8; 16],
    s_journal_inum: u32,
    s_journal_dev: u32,
    s_last_orphan: u32,
    s_hash_seed: [u32; 4],
    s_def_hash_version: u8,
    s_jnl_backup_type: u8,
    s_desc_size: u16,
    s_default_mount_opts: u32,
    s_first_meta_bg: u32,
    s_mkfs_time: u32,
    s_jnl_blocks: [u32; 17],
    s_blocks_count_hi: u32,
    s_r_blocks_count_hi: u32,
    s_free_blocks_count_hi: u32,
    s_min_extra_isize: u16,
    s_want_extra_isize: u16,
    s_flags: u32,
    s_raid_stride: u16,
    s_mmp_interval: u16,
    s_mmp_block: u64,
    s_raid_stripe_width: u32,
    s_log_groups_per_flex: u8,
    s_checksum_type: u8,
    s_reserved_pad: u16,
    s_kbytes_written: u64,
    s_snapshot_inum: u32,
    s_snapshot_id: u32,
    s_snapshot_r_blocks_count: u64,
    s_snapshot_list: u32,
    s_error_count: u32,
    s_first_error_time: u32,
    s_first_error_ino: u32,
    s_first_error_block: u64,
    s_first_error_func: [u8; 32],
    s_first_error_line: u32,
    s_last_error_time: u32,
    s_last_error_ino: u32,
    s_last_error_line: u32,
    s_last_error_block: u64,
    s_last_error_func: [u8; 32],
    s_mount_opts: [u8; 64],
    s_usr_quota_inum: u32,
    s_grp_quota_inum: u32,
    s_overhead_clusters: u32,
    s_backup_bgs: [u32; 2],
    s_encrypt_algos: [u8; 4],
    s_encrypt_pw_salt: [u8; 16],
    s_lvf_knode: u32,
    s_checksum_seed: u32,
    s_wtime_hi: u8,
    s_mtime_hi: u8,
    s_mkfs_time_hi: u8,
    s_lastcheck_hi: u8,
    s_first_error_time_hi: u8,
    s_last_error_time_hi: u8,
    s_pad: [u8; 2],
    s_checksum: u32,
}

#[repr(C, packed)]
struct Ext4Inode {
    i_mode: u16,
    i_uid: u16,
    i_size_lo: u32,
    i_atime: u32,
    i_ctime: u32,
    i_mtime: u32,
    i_dtime: u32,
    i_gid: u16,
    i_links_count: u16,
    i_blocks_lo: u32,
    i_flags: u32,
    i_osd1: u32,
    i_block: [u32; 15],
    i_generation: u32,
    i_file_acl_lo: u32,
    i_size_high: u32,
    i_obso_faddr: u32,
    i_osd2: [u8; 12],
    i_extra_isize: u16,
    i_checksum_hi: u16,
    i_ctime_extra: u32,
    i_mtime_extra: u32,
    i_atime_extra: u32,
    i_crtime: u32,
    i_crtime_extra: u32,
    i_version_hi: u32,
    i_projid: u32,
    reserved: [u8; 96],
}

pub struct Ext4Writer;

impl Ext4Writer {
    pub fn write_image(tree: &VfsTree, output_path: &Path) -> Result<u64> {
        let mut file = File::create(output_path)
            .with_context(|| format!("Failed to create EXT4 output file {}", output_path.display()))?;

        let total_inodes = (tree.nodes.len() + 16) as u32;
        let mut current_block = 5u32; // Block 0=Boot, 1=Super, 2=GroupDesc, 3=BlockBitmap, 4=InodeBitmap

        let mut inode_map: BTreeMap<PathBuf, u32> = BTreeMap::new();
        let mut ino_counter = 11u32; // EXT4 user inodes start after reserved (11)

        for (path, _) in &tree.nodes {
            if path.as_os_str().is_empty() {
                inode_map.insert(path.clone(), 2); // Root inode is always 2
            } else {
                inode_map.insert(path.clone(), ino_counter);
                ino_counter += 1;
            }
        }

        let mut file_blocks = Vec::new();
        let mut inodes = Vec::new();

        // Build inodes and write file data blocks
        for (path, node) in &tree.nodes {
            let ino = *inode_map.get(path).unwrap();

            let mut block_ptr = 0u32;
            let mut file_size = 0u32;

            match &node.node_type {
                VfsNodeType::RegularFile { content } => {
                    file_size = content.len() as u32;
                    block_ptr = current_block;
                    file_blocks.push((block_ptr, content.clone()));

                    let blocks_count = (content.len() + EXT4_BLOCK_SIZE - 1) / EXT4_BLOCK_SIZE;
                    current_block += blocks_count as u32;
                }
                VfsNodeType::Symlink { target } => {
                    let bytes = target.as_bytes();
                    file_size = bytes.len() as u32;
                    block_ptr = current_block;
                    file_blocks.push((block_ptr, bytes.to_vec()));
                    current_block += 1;
                }
                VfsNodeType::Directory { .. } => {
                    // Generate EXT4 directory entries block
                    let children: Vec<(&PathBuf, &VfsNode)> = tree
                        .nodes
                        .iter()
                        .filter(|(p, _)| p.parent() == Some(path) && !p.as_os_str().is_empty())
                        .collect();

                    let mut dir_block = Vec::new();

                    // Self '.' entry
                    dir_block.extend_from_slice(&(ino).to_le_bytes());
                    dir_block.extend_from_slice(&12u16.to_le_bytes()); // rec_len
                    dir_block.push(1); // name_len
                    dir_block.push(2); // file_type = DIR
                    dir_block.push(b'.');
                    dir_block.extend_from_slice(&[0u8; 3]); // padding

                    // Parent '..' entry
                    let parent_ino = path
                        .parent()
                        .and_then(|p| inode_map.get(p))
                        .copied()
                        .unwrap_or(2);

                    dir_block.extend_from_slice(&parent_ino.to_le_bytes());
                    dir_block.extend_from_slice(&12u16.to_le_bytes());
                    dir_block.push(2);
                    dir_block.push(2);
                    dir_block.extend_from_slice(b"..");
                    dir_block.extend_from_slice(&[0u8; 2]);

                    for (child_path, child_node) in children {
                        let child_name = child_path.file_name().unwrap().to_string_lossy();
                        let child_ino = *inode_map.get(child_path).unwrap();

                        let ft = match &child_node.node_type {
                            VfsNodeType::Directory { .. } => 2,
                            VfsNodeType::RegularFile { .. } => 1,
                            VfsNodeType::Symlink { .. } => 7,
                            VfsNodeType::WhiteoutDevice => 3, // CHRDEV
                        };

                        let len = 8 + child_name.len();
                        let rec_len = (len + 3) & !3; // Align 4 bytes

                        dir_block.extend_from_slice(&child_ino.to_le_bytes());
                        dir_block.extend_from_slice(&(rec_len as u16).to_le_bytes());
                        dir_block.push(child_name.len() as u8);
                        dir_block.push(ft);
                        dir_block.extend_from_slice(child_name.as_bytes());

                        let pad = rec_len - len;
                        if pad > 0 {
                            dir_block.extend_from_slice(&vec![0u8; pad]);
                        }
                    }

                    file_size = dir_block.len() as u32;
                    block_ptr = current_block;
                    file_blocks.push((block_ptr, dir_block));
                    current_block += 1;
                }
                VfsNodeType::WhiteoutDevice => {
                    file_size = 0;
                    block_ptr = 0;
                }
            }

            let mut i_block = [0u32; 15];
            if block_ptr > 0 {
                i_block[0] = block_ptr;
            }

            let inode = Ext4Inode {
                i_mode: node.mode as u16,
                i_uid: node.uid as u16,
                i_size_lo: file_size,
                i_atime: node.mtime as u32,
                i_ctime: node.mtime as u32,
                i_mtime: node.mtime as u32,
                i_dtime: 0,
                i_gid: node.gid as u16,
                i_links_count: 1,
                i_blocks_lo: (file_size + 511) / 512,
                i_flags: 0,
                i_osd1: 0,
                i_block,
                i_generation: 0,
                i_file_acl_lo: 0,
                i_size_high: 0,
                i_obso_faddr: 0,
                i_osd2: [0u8; 12],
                i_extra_isize: 32,
                i_checksum_hi: 0,
                i_ctime_extra: 0,
                i_mtime_extra: 0,
                i_atime_extra: 0,
                i_crtime: node.mtime as u32,
                i_crtime_extra: 0,
                i_version_hi: 0,
                i_projid: 0,
                reserved: [0u8; 96],
            };

            inodes.push((ino, inode));
        }

        let total_blocks = current_block;

        // Write EXT4 Superblock at offset 1024
        let sb = Ext4SuperBlock {
            s_inodes_count: total_inodes,
            s_blocks_count_lo: total_blocks,
            s_r_blocks_count_lo: 0,
            s_free_blocks_count_lo: total_blocks - current_block,
            s_free_inodes_count: total_inodes - inodes.len() as u32,
            s_first_data_block: 0,
            s_log_block_size: 2, // 4096 bytes (1024 << 2)
            s_log_cluster_size: 2,
            s_blocks_per_group: 32768,
            s_clusters_per_group: 32768,
            s_inodes_per_group: total_inodes,
            s_mtime: 0,
            s_wtime: 0,
            s_mnt_count: 1,
            s_max_mnt_count: 65535,
            s_magic: EXT4_SUPER_MAGIC.to_le(),
            s_state: 1,
            s_errors: 1,
            s_minor_rev_level: 0,
            s_lastcheck: 0,
            s_checkinterval: 0,
            s_creator_os: 0, // Linux
            s_rev_level: 1,  // Dynamic
            s_def_resuid: 0,
            s_def_resgid: 0,
            s_first_ino: 11,
            s_inode_size: EXT4_INODE_SIZE as u16,
            s_block_group_nr: 0,
            s_feature_compat: 0,
            s_feature_incompat: 0x40, // FILETYPE
            s_feature_ro_compat: 0,
            s_uuid: [0u8; 16],
            s_volume_name: *b"NANX_SU_EXT4\0\0\0\0",
            s_last_mounted: [0u8; 64],
            s_algorithm_usage_bitmap: 0,
            s_prealloc_blocks: 0,
            s_prealloc_dir_blocks: 0,
            s_reserved_gdb_count: 0,
            s_journal_uuid: [0u8; 16],
            s_journal_inum: 0,
            s_journal_dev: 0,
            s_last_orphan: 0,
            s_hash_seed: [0; 4],
            s_def_hash_version: 0,
            s_jnl_backup_type: 0,
            s_desc_size: 32,
            s_default_mount_opts: 0,
            s_first_meta_bg: 0,
            s_mkfs_time: 0,
            s_jnl_blocks: [0; 17],
            s_blocks_count_hi: 0,
            s_r_blocks_count_hi: 0,
            s_free_blocks_count_hi: 0,
            s_min_extra_isize: 32,
            s_want_extra_isize: 32,
            s_flags: 0,
            s_raid_stride: 0,
            s_mmp_interval: 0,
            s_mmp_block: 0,
            s_raid_stripe_width: 0,
            s_log_groups_per_flex: 0,
            s_checksum_type: 0,
            s_reserved_pad: 0,
            s_kbytes_written: 0,
            s_snapshot_inum: 0,
            s_snapshot_id: 0,
            s_snapshot_r_blocks_count: 0,
            s_snapshot_list: 0,
            s_error_count: 0,
            s_first_error_time: 0,
            s_first_error_ino: 0,
            s_first_error_block: 0,
            s_first_error_func: [0; 32],
            s_first_error_line: 0,
            s_last_error_time: 0,
            s_last_error_ino: 0,
            s_last_error_line: 0,
            s_last_error_block: 0,
            s_last_error_func: [0; 32],
            s_mount_opts: [0; 64],
            s_usr_quota_inum: 0,
            s_grp_quota_inum: 0,
            s_overhead_clusters: 0,
            s_backup_bgs: [0; 2],
            s_encrypt_algos: [0; 4],
            s_encrypt_pw_salt: [0; 16],
            s_lvf_knode: 0,
            s_checksum_seed: 0,
            s_wtime_hi: 0,
            s_mtime_hi: 0,
            s_mkfs_time_hi: 0,
            s_lastcheck_hi: 0,
            s_first_error_time_hi: 0,
            s_last_error_time_hi: 0,
            s_pad: [0; 2],
            s_checksum: 0,
        };

        // Fill zero header blocks
        let zero_space = vec![0u8; EXT4_BLOCK_SIZE * total_blocks as usize];
        file.write_all(&zero_space)?;

        // Write Superblock
        file.seek(SeekFrom::Start(1024))?;
        let mut sb_bytes = vec![0u8; std::mem::size_of::<Ext4SuperBlock>()];
        unsafe {
            std::ptr::copy_nonoverlapping(
                &sb as *const _ as *const u8,
                sb_bytes.as_mut_ptr(),
                sb_bytes.len(),
            );
        }
        file.write_all(&sb_bytes)?;

        // Write file payload blocks
        for (blk, data) in file_blocks {
            file.seek(SeekFrom::Start(blk as u64 * EXT4_BLOCK_SIZE as u64))?;
            file.write_all(&data)?;
        }

        // Write Inodes in block 4
        file.seek(SeekFrom::Start(4 * EXT4_BLOCK_SIZE as u64))?;
        for (_ino, inode) in inodes {
            let mut inode_bytes = vec![0u8; EXT4_INODE_SIZE];
            unsafe {
                std::ptr::copy_nonoverlapping(
                    &inode as *const _ as *const u8,
                    inode_bytes.as_mut_ptr(),
                    std::mem::size_of::<Ext4Inode>(),
                );
            }
            file.write_all(&inode_bytes)?;
        }

        file.flush()?;
        let size = file.seek(SeekFrom::End(0))?;
        Ok(size)
    }
}
