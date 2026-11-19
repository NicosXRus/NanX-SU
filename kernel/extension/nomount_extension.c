// SPDX-License-Identifier: GPL-2.0
#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/uaccess.h>
#include <linux/mutex.h>
#include <linux/path.h>
#include <linux/namei.h>
#include "nomount_extension.h"

bool nomount_enabled = false;
static DEFINE_MUTEX(nomount_lock);

struct nomount_redirect_rule {
    char target_path[256];
    char module_path[256];
    struct list_head list;
};

static LIST_HEAD(nomount_rules);

int ksu_extension_nomount_get(u32 id, u64 *value)
{
    (void)id;
    mutex_lock(&nomount_lock);
    *value = nomount_enabled ? 1 : 0;
    mutex_unlock(&nomount_lock);
    return 0;
}

int ksu_extension_nomount_set(u32 id, u64 value)
{
    (void)id;
    mutex_lock(&nomount_lock);
    nomount_enabled = (value != 0);
    mutex_unlock(&nomount_lock);
    pr_info("nomount: extension set to %d\n", nomount_enabled);
    return 0;
}

struct nomount_rule_cmd {
    char target[256];
    char module[256];
};

int ksu_extension_nomount_io(u32 id, void __user *raw)
{
    struct nomount_rule_cmd cmd;
    struct nomount_redirect_rule *rule;
    (void)id;

    if (!raw)
        return -EINVAL;

    if (copy_from_user(&cmd, raw, sizeof(cmd)))
        return -EFAULT;

    mutex_lock(&nomount_lock);
    rule = kzalloc(sizeof(*rule), GFP_KERNEL);
    if (!rule) {
        mutex_unlock(&nomount_lock);
        return -ENOMEM;
    }

    strscpy(rule->target_path, cmd.target, sizeof(rule->target_path));
    strscpy(rule->module_path, cmd.module, sizeof(rule->module_path));
    list_add_tail(&rule->list, &nomount_rules);

    mutex_unlock(&nomount_lock);
    pr_info("nomount: added rule %s -> %s\n", cmd.target, cmd.module);
    return 0;
}

int ksu_extension_nomount_wipe(u32 id)
{
    struct nomount_redirect_rule *rule, *tmp;
    (void)id;

    mutex_lock(&nomount_lock);
    list_for_each_entry_safe(rule, tmp, &nomount_rules, list) {
        list_del(&rule->list);
        kfree(rule);
    }
    nomount_enabled = false;
    mutex_unlock(&nomount_lock);

    pr_info("nomount: wiped all rules\n");
    return 0;
}
