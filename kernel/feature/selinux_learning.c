// SPDX-License-Identifier: GPL-2.0
#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/mutex.h>
#include <linux/list.h>
#include <linux/cred.h>

bool selinux_learning_enabled = false;
static DEFINE_MUTEX(selinux_learning_lock);

struct selinux_denial_record {
    uid_t uid;
    char scontext[128];
    char tcontext[128];
    char tclass[64];
    char permission[64];
    struct list_head list;
};

static LIST_HEAD(denial_records);

int ksu_selinux_learning_add_denial(uid_t uid, const char *scontext, const char *tcontext, const char *tclass, const char *perm)
{
    struct selinux_denial_record *rec;

    if (!selinux_learning_enabled)
        return 0;

    mutex_lock(&selinux_learning_lock);
    rec = kzalloc(sizeof(*rec), GFP_KERNEL);
    if (!rec) {
        mutex_unlock(&selinux_learning_lock);
        return -ENOMEM;
    }

    rec->uid = uid;
    strscpy(rec->scontext, scontext ? scontext : "u:r:untrusted_app:s0", sizeof(rec->scontext));
    strscpy(rec->tcontext, tcontext ? tcontext : "u:object_r:system_file:s0", sizeof(rec->tcontext));
    strscpy(rec->tclass, tclass ? tclass : "file", sizeof(rec->tclass));
    strscpy(rec->permission, perm ? perm : "read", sizeof(rec->permission));

    list_add_tail(&rec->list, &denial_records);
    mutex_unlock(&selinux_learning_lock);

    pr_info("selinux_learning: recorded denial for uid %d: allow %s %s:%s %s\n",
            uid, rec->scontext, rec->tcontext, rec->tclass, rec->permission);
    return 0;
}
