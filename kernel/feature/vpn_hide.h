/* SPDX-License-Identifier: GPL-2.0 */
#ifndef __KSU_H_VPN_HIDE
#define __KSU_H_VPN_HIDE

#include <linux/types.h>
#include <linux/compiler.h>

extern bool vpn_hide_enabled;

int ksu_vpn_hide_add_uid(uid_t uid);
int ksu_vpn_hide_remove_uid(uid_t uid);
bool ksu_vpn_hide_should_hide_uid(uid_t uid);
bool ksu_vpn_hide_is_vpn_iface(const char *ifname);

#endif
