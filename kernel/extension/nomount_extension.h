/* SPDX-License-Identifier: GPL-2.0 */
#ifndef __KSU_H_NOMOUNT_EXTENSION
#define __KSU_H_NOMOUNT_EXTENSION

#include <linux/types.h>
#include <linux/compiler.h>

extern bool nomount_enabled;

int ksu_extension_nomount_get(u32 id, u64 *value);
int ksu_extension_nomount_set(u32 id, u64 value);
int ksu_extension_nomount_io(u32 id, void __user *raw);
int ksu_extension_nomount_wipe(u32 id);

#endif
