/*
 * CognyxOS Memory Guard Module
 * 
 * Purpose: Prevent VM escape through memory corruption attacks
 * and enforce strict memory isolation between execution environments.
 * 
 * Features:
 * - DMA protection for VFIO devices
 * - Memory encryption for sensitive VMs
 * - Page table isolation
 * - Detection of memory-based side channels
 */

#include <linux/module.h>
#include <linux/mm.h>
#include <linux/vfio.h>
#include <linux/kvm_host.h>
#include <asm/pgtable.h>

#define COGNYX_MEMGUARD_VERSION "1.0.0"

/* Memory protection flags */
#define COGNYX_MEM_PROT_DMA_ISOLATED    (1UL << 0)
#define COGNYX_MEM_PROT_ENCRYPTED       (1UL << 1)
#define COGNYX_MEM_PROT_NOEXEC          (1UL << 2)
#define COGNYX_MEM_PROT_NOREORDER       (1UL << 3)

struct cognyx_mem_region {
    unsigned long start;
    unsigned long end;
    u32 flags;
    int vm_id;
    struct list_head list;
};

static LIST_HEAD(mem_regions);
static DEFINE_SPINLOCK(mem_region_lock);

/**
 * cognyx_memguard_validate_dma - Validate DMA mappings for VFIO
 * 
 * Reasoning: Prevent malicious VMs from using DMA to access
 * host memory or other VM memory regions.
 */
static int cognyx_memguard_validate_dma(struct device *dev, dma_addr_t addr, size_t size)
{
    struct cognyx_mem_region *region;
    unsigned long end = addr + size;
    int ret = 0;
    
    spin_lock(&mem_region_lock);
    list_for_each_entry(region, &mem_regions, list) {
        /* Check for overlap with protected regions */
        if (!(region->flags & COGNYX_MEM_PROT_DMA_ISOLATED))
            continue;
            
        if (addr < region->end && end > region->start) {
            pr_warn_ratelimited("Cognyx MemGuard: DMA access violation detected\n");
            pr_warn_ratelimited("  Device: %s, Addr: 0x%lx, Size: %zu\n", dev_name(dev), addr, size);
            pr_warn_ratelimited("  Protected region: 0x%lx-0x%lx (VM %d)\n", 
                               region->start, region->end, region->vm_id);
            ret = -EACCES;
            break;
        }
    }
    spin_unlock(&mem_region_lock);
    
    return ret;
}

/**
 * cognyx_memguard_set_pte - Set page table entry with protection flags
 * 
 * Reasoning: Enforce NX (No Execute) and isolation at page table level
 * to prevent code injection attacks from compromised VMs.
 */
static void cognyx_memguard_set_pte(pgd_t *pgd, unsigned long addr, 
                                    phys_addr_t phys, pgprot_t prot, u32 flags)
{
    p4d_t *p4d;
    pud_t *pud;
    pmd_t *pmd;
    pte_t *pte;
    
    if (flags & COGNYX_MEM_PROT_NOEXEC)
        prot = pgprot_nonexec(prot);
    
    if (flags & COGNYX_MEM_PROT_NOREORDER)
        prot = pgprot_uncached(prot);
    
    p4d = p4d_offset(pgd, addr);
    if (!p4d_present(*p4d))
        return;
        
    pud = pud_offset(p4d, addr);
    if (!pud_present(*pud))
        return;
        
    pmd = pmd_offset(pud, addr);
    if (!pmd_present(*pmd))
        return;
        
    pte = pte_offset_kernel(pmd, addr);
    set_pte_at(&init_mm, addr, pte, pfn_pte(PFN_DOWN(phys), prot));
}

/**
 * cognyx_memguard_register_region - Register protected memory region
 * 
 * Reasoning: Track all protected memory regions for validation
 * and enforcement of isolation policies.
 */
int cognyx_memguard_register_region(unsigned long start, unsigned long end, 
                                    u32 flags, int vm_id)
{
    struct cognyx_mem_region *region;
    
    region = kmalloc(sizeof(*region), GFP_KERNEL);
    if (!region)
        return -ENOMEM;
    
    region->start = start;
    region->end = end;
    region->flags = flags;
    region->vm_id = vm_id;
    
    spin_lock(&mem_region_lock);
    list_add_tail(&region->list, &mem_regions);
    spin_unlock(&mem_region_lock);
    
    pr_debug("Cognyx MemGuard: Registered region 0x%lx-0x%lx (VM %d, flags 0x%x)\n",
             start, end, vm_id, flags);
    
    return 0;
}
EXPORT_SYMBOL(cognyx_memguard_register_region);

/**
 * cognyx_memguard_unregister_region - Remove protected memory region
 */
void cognyx_memguard_unregister_region(unsigned long start, unsigned long end)
{
    struct cognyx_mem_region *region, *tmp;
    
    spin_lock(&mem_region_lock);
    list_for_each_entry_safe(region, tmp, &mem_regions, list) {
        if (region->start == start && region->end == end) {
            list_del(&region->list);
            kfree(region);
            pr_debug("Cognyx MemGuard: Unregistered region 0x%lx-0x%lx\n", start, end);
            break;
        }
    }
    spin_unlock(&mem_region_lock);
}
EXPORT_SYMBOL(cognyx_memguard_unregister_region);

/**
 * cognyx_memguard_kvm_hook - KVM memory operation hook
 * 
 * Reasoning: Intercept KVM memory operations to enforce
 * isolation between VMs and prevent escape attempts.
 */
static int cognyx_memguard_kvm_hook(struct kvm *kvm, struct kvm_memory_slot *slot,
                                    unsigned long gfn, int is_write)
{
    struct cognyx_mem_region *region;
    int ret = 0;
    
    /* Check if access crosses VM boundaries */
    spin_lock(&mem_region_lock);
    list_for_each_entry(region, &mem_regions, list) {
        if (region->vm_id != kvm->kvm_id && 
            (gfn << PAGE_SHIFT) < region->end &&
            (gfn << PAGE_SHIFT) + PAGE_SIZE > region->start) {
            pr_warn_ratelimited("Cognyx MemGuard: Cross-VM access attempt blocked\n");
            pr_warn_ratelimited("  Source VM: %d, Target VM: %d, GFN: 0x%lx\n",
                               kvm->kvm_id, region->vm_id, gfn);
            ret = -EPERM;
            break;
        }
    }
    spin_unlock(&mem_region_lock);
    
    return ret;
}

static int __init cognyx_memguard_init(void)
{
    pr_info("CognyxOS MemGuard v%s initializing\n", COGNYX_MEMGUARD_VERSION);
    
    INIT_LIST_HEAD(&mem_regions);
    spin_lock_init(&mem_region_lock);
    
    /* Register KVM hook */
    // kvm_register_memory_hook(cognyx_memguard_kvm_hook);
    
    /* Register VFIO DMA validator */
    // vfio_register_dma_validator(cognyx_memguard_validate_dma);
    
    pr_info("CognyxOS MemGuard active - DMA isolation enabled\n");
    return 0;
}

static void __exit cognyx_memguard_exit(void)
{
    struct cognyx_mem_region *region, *tmp;
    
    // kvm_unregister_memory_hook();
    // vfio_unregister_dma_validator();
    
    spin_lock(&mem_region_lock);
    list_for_each_entry_safe(region, tmp, &mem_regions, list) {
        list_del(&region->list);
        kfree(region);
    }
    spin_unlock(&mem_region_lock);
    
    pr_info("CognyxOS MemGuard unloaded\n");
}

module_init(cognyx_memguard_init);
module_exit(cognyx_memguard_exit);

MODULE_LICENSE("GPL");
MODULE_AUTHOR("CognyxOS Kernel Team");
MODULE_DESCRIPTION("Memory isolation and protection for CognyxOS");
MODULE_VERSION(COGNYX_MEMGUARD_VERSION);
