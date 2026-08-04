/*
 * CognyxOS Custom Scheduler Extension
 * 
 * Purpose: Extend Linux CFS with intent-aware priority classes
 * for AI agent workloads and VM scheduling.
 * 
 * This module adds:
 * - Real-time priority class for capability execution
 * - Interactive priority for user-facing operations  
 * - Background priority for batch tasks
 * - VM-aware scheduling to prevent host starvation
 */

#include <linux/sched.h>
#include <linux/module.h>
#include <linux/kprobes.h>

#define COGNYX_SCHED_VERSION "1.0.0"

/* Priority classes for CognyxOS workloads */
enum cognyx_priority_class {
    COGNYX_PRIO_REALTIME = 0,    /* Capability execution, GPU access */
    COGNYX_PRIO_INTERACTIVE = 1, /* User-facing operations */
    COGNYX_PRIO_NORMAL = 2,      /* Standard VM workloads */
    COGNYX_PRIO_BACKGROUND = 3,  /* Batch tasks, updates */
    COGNYX_PRIO_COUNT
};

/* Per-CPU runqueue extension */
struct cognyx_rq {
    u64 runtime_ns[COGNYX_PRIO_COUNT];
    u64 last_switch;
    int current_priority;
};

static struct cognyx_rq __percpu *cognyx_rq;

/**
 * cognyx_select_task_rq - Select optimal CPU for task
 * 
 * Reasoning: Agent workloads have different characteristics than
 * traditional processes. This function places real-time capability
 * execution on isolated CPUs to minimize latency.
 */
static int cognyx_select_task_rq(struct task_struct *p, int prev_cpu, int sd_flag, int wake_flags)
{
    struct cognyx_rq *rq = this_cpu_ptr(cognyx_rq);
    
    /* Pin real-time capability tasks to isolated CPUs */
    if (p->prio <= 50 && (p->flags & PF_COGNYX_CAPABILITY)) {
        return cpumask_first(cpu_isolated_map);
    }
    
    return prev_cpu;
}

/**
 * cognyx_entity_enqueue - Enqueue task with priority tracking
 * 
 * Reasoning: Track runtime per priority class to enable
 * fair scheduling between VMs and host services.
 */
static void cognyx_entity_enqueue(struct task_struct *p)
{
    struct cognyx_rq *rq = this_cpu_ptr(cognyx_rq);
    enum cognyx_priority_class prio;
    
    /* Determine priority class from task flags or cgroup */
    if (p->flags & PF_COGNYX_CAPABILITY)
        prio = COGNYX_PRIO_REALTIME;
    else if (p->flags & PF_COGNYX_INTERACTIVE)
        prio = COGNYX_PRIO_INTERACTIVE;
    else if (p->flags & PF_COGNYX_BACKGROUND)
        prio = COGNYX_PRIO_BACKGROUND;
    else
        prio = COGNYX_PRIO_NORMAL;
    
    rq->runtime_ns[prio] += p->se.sum_exec_runtime;
    rq->current_priority = prio;
}

/**
 * cognyx_check_preempt_wakeup - Preemption logic for capability tasks
 * 
 * Reasoning: Real-time capability execution must preempt normal
 * workloads to maintain sub-10μs IPC latency targets.
 */
static void cognyx_check_preempt_wakeup(struct rq *rq, struct task_struct *p, int wake_flags)
{
    struct task_struct *curr = rq->curr;
    
    if (!curr || curr == p)
        return;
    
    /* Capability tasks always preempt normal workloads */
    if ((p->flags & PF_COGNYX_CAPABILITY) && !(curr->flags & PF_COGNYX_CAPABILITY)) {
        resched_curr(rq);
        return;
    }
    
    /* VM vCPUs should not starve host services */
    if ((curr->flags & PF_VCPU) && !(p->flags & PF_VCPU)) {
        u64 vm_runtime = curr->se.sum_exec_runtime;
        if (vm_runtime > NSEC_PER_MSEC * 10) /* 10ms max continuous VM run */
            resched_curr(rq);
    }
}

static struct sched_ext_ops cognyx_sched_ops = {
    .name = "cognyx",
    .select_cpu = cognyx_select_task_rq,
    .enqueue = cognyx_entity_enqueue,
    .check_preempt_wakeup = cognyx_check_preempt_wakeup,
};

static int __init cognyx_sched_init(void)
{
    int cpu;
    
    pr_info("CognyxOS Scheduler v%s loading\n", COGNYX_SCHED_VERSION);
    
    /* Allocate per-CPU runqueue extensions */
    cognyx_rq = alloc_percpu(struct cognyx_rq);
    if (!cognyx_rq)
        return -ENOMEM;
    
    for_each_possible_cpu(cpu) {
        struct cognyx_rq *rq = per_cpu_ptr(cognyx_rq, cpu);
        memset(rq, 0, sizeof(*rq));
    }
    
    /* Register with BPF scheduler framework */
    // scx_register_ops(&cognyx_sched_ops);
    
    pr_info("CognyxOS Scheduler initialized\n");
    return 0;
}

static void __exit cognyx_sched_exit(void)
{
    // scx_unregister_ops(&cognyx_sched_ops);
    free_percpu(cognyx_rq);
    pr_info("CognyxOS Scheduler unloaded\n");
}

module_init(cognyx_sched_init);
module_exit(cognyx_sched_exit);

MODULE_LICENSE("GPL");
MODULE_AUTHOR("CognyxOS Kernel Team");
MODULE_DESCRIPTION("Intent-aware scheduler for AI agent workloads");
MODULE_VERSION(COGNYX_SCHED_VERSION);
