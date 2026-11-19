// SPDX-License-Identifier: GPL-2.0
#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/string.h>
#include <linux/mutex.h>
#include <linux/list.h>
#include "vpn_hide.h"

bool vpn_hide_enabled = false;
static DEFINE_MUTEX(vpn_hide_lock);

struct vpn_hide_uid_entry {
    uid_t uid;
    struct list_head list;
};

static LIST_HEAD(vpn_hide_uids);

static const char *const vpn_iface_prefixes[] = {
    "tun", "wg", "ppp", "tap", "p2p", "clat", "wireguard", NULL
};

bool ksu_vpn_hide_is_vpn_iface(const char *ifname)
{
    int i;
    if (!ifname)
        return false;

    for (i = 0; vpn_iface_prefixes[i] != NULL; i++) {
        if (strncmp(ifname, vpn_iface_prefixes[i], strlen(vpn_iface_prefixes[i])) == 0)
            return true;
    }
    return false;
}

bool ksu_vpn_hide_should_hide_uid(uid_t uid)
{
    struct vpn_hide_uid_entry *entry;
    bool found = false;

    if (!vpn_hide_enabled)
        return false;

    mutex_lock(&vpn_hide_lock);
    list_for_each_entry(entry, &vpn_hide_uids, list) {
        if (entry->uid == uid) {
            found = true;
            break;
        }
    }
    mutex_unlock(&vpn_hide_lock);
    return found;
}

int ksu_vpn_hide_add_uid(uid_t uid)
{
    struct vpn_hide_uid_entry *entry;

    mutex_lock(&vpn_hide_lock);
    list_for_each_entry(entry, &vpn_hide_uids, list) {
        if (entry->uid == uid) {
            mutex_unlock(&vpn_hide_lock);
            return 0; // Already added
        }
    }

    entry = kzalloc(sizeof(*entry), GFP_KERNEL);
    if (!entry) {
        mutex_unlock(&vpn_hide_lock);
        return -ENOMEM;
    }

    entry->uid = uid;
    list_add_tail(&entry->list, &vpn_hide_uids);
    vpn_hide_enabled = true;
    mutex_unlock(&vpn_hide_lock);

    pr_info("vpn_hide: added uid %d\n", uid);
    return 0;
}

int ksu_vpn_hide_remove_uid(uid_t uid)
{
    struct vpn_hide_uid_entry *entry, *tmp;

    mutex_lock(&vpn_hide_lock);
    list_for_each_entry_safe(entry, tmp, &vpn_hide_uids, list) {
        if (entry->uid == uid) {
            list_del(&entry->list);
            kfree(entry);
            break;
        }
    }
    vpn_hide_enabled = !list_empty(&vpn_hide_uids);
    mutex_unlock(&vpn_hide_lock);

    pr_info("vpn_hide: removed uid %d\n", uid);
    return 0;
}
