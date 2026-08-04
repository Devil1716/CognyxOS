#!/bin/bash
# CognyxOS Host Build Script
# Builds minimal immutable host image for bare-metal deployment

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${SCRIPT_DIR}/build"
OUTPUT_DIR="${SCRIPT_DIR}/output"
KERNEL_VERSION="6.8.9"
HOST_VERSION="1.0.0"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Create directories
mkdir -p "${BUILD_DIR}" "${OUTPUT_DIR}"

log_info "Building CognyxOS Host v${HOST_VERSION}"
log_info "Target kernel: ${KERNEL_VERSION}"

# ============================================
# Step 1: Download and prepare kernel
# ============================================
log_info "Step 1: Preparing Linux kernel..."

KERNEL_SRC="${BUILD_DIR}/linux-${KERNEL_VERSION}"
if [ ! -d "${KERNEL_SRC}" ]; then
    log_info "Downloading kernel source..."
    wget -q "https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-${KERNEL_VERSION}.tar.xz" \
        -O "${BUILD_DIR}/linux-${KERNEL_VERSION}.tar.xz"
    tar -xf "${BUILD_DIR}/linux-${KERNEL_VERSION}.tar.xz" -C "${BUILD_DIR}"
fi

cd "${KERNEL_SRC}"

# Configure kernel with minimal options for CognyxOS
log_info "Configuring kernel..."
make defconfig
./scripts/config --disable CONFIG_X86_PTDUMP
./scripts/config --disable CONFIG_DEBUG_KERNEL
./scripts/config --enable CONFIG_KVM
./scripts/config --enable CONFIG_VFIO
./scripts/config --enable CONFIG_VFIO_PCI
./scripts/config --enable CONFIG_VHOST_NET
./scripts/config --enable CONFIG_VIRTIO
./scripts/config --enable CONFIG_VIRTIO_PCI
./scripts/config --enable CONFIG_VSOCKETS
./scripts/config --enable CONFIG_VHOST_VSOCK
./scripts/config --enable CONFIG_ZSWAP
./scripts/config --enable CONFIG_ZSTD_DECOMPRESS
./scripts/config --enable CONFIG_BTRFS_FS
./scripts/config --module CONFIG_ZFS
./scripts/config --enable CONFIG_CGROUPS
./scripts/config --enable CONFIG_CGROUP_BPF
./scripts/config --enable CONFIG_MEMCG
./scripts/config --enable CONFIG_BLK_CGROUP
./scripts/config --enable CONFIG_CGROUP_SCHED
./scripts/config --enable CONFIG_SECURITY
./scripts/config --enable CONFIG_SECURITY_APPARMOR
./scripts/config --enable CONFIG_SECURITY_SELINUX
./scripts/config --enable CONFIG_IOMMU_SUPPORT
./scripts/config --enable CONFIG_INTEL_IOMMU
./scripts/config --enable CONFIG_AMD_IOMMU
./scripts/config --enable CONFIG_GPU_IOMMU
./scripts/config --enable CONFIG_DRM
./scripts/config --enable CONFIG_DRM_AMDGPU
./scripts/config --enable CONFIG_DRM_NOUVEAU
./scripts/config --enable CONFIG_NF_TABLES
./scripts/config --enable CONFIG_NFT_CHAIN_NAT
./scripts/config --enable CONFIG_NETFILTER_XT_TARGET_MASQUERADE

# Build kernel
log_info "Compiling kernel (this may take a while)..."
make -j$(nproc) bzImage modules

# Install kernel to build directory
log_info "Installing kernel modules..."
make INSTALL_MOD_PATH="${BUILD_DIR}/root" modules_install
make INSTALL_MOD_PATH="${BUILD_DIR}/root" firmware_install

cp arch/x86/boot/bzImage "${BUILD_DIR}/vmlinuz-cognyx"

# ============================================
# Step 2: Build initramfs
# ============================================
log_info "Step 2: Building initramfs..."

INITRAMFS_ROOT="${BUILD_DIR}/initramfs-root"
mkdir -p "${INITRAMFS_ROOT}"/{bin,sbin,usr/bin,usr/sbin,etc,dev,proc,sys,mnt,var/run}

# Copy essential binaries
cp /bin/busybox "${INITRAMFS_ROOT}/bin/" || {
    log_warn "busybox not found, installing..."
    apt-get update && apt-get install -y busybox-static
    cp /bin/busybox "${INITRAMFS_ROOT}/bin/"
}

# Create symlinks for busybox
cd "${INITRAMFS_ROOT}/bin"
for cmd in sh ls cat mount mkdir mknod insmod modprobe rm ip dmesg; do
    ln -sf busybox "$cmd" 2>/dev/null || true
done
cd "${SCRIPT_DIR}"

# Copy kernel modules needed at boot
mkdir -p "${INITRAMFS_ROOT}/lib/modules/${KERNEL_VERSION}"
cp -r "${BUILD_DIR}/root/lib/modules/${KERNEL_VERSION}/kernel/drivers/virtio" \
    "${INITRAMFS_ROOT}/lib/modules/${KERNEL_VERSION}/kernel/drivers/" || true
cp -r "${BUILD_DIR}/root/lib/modules/${KERNEL_VERSION}/kernel/drivers/vhost" \
    "${INITRAMFS_ROOT}/lib/modules/${KERNEL_VERSION}/kernel/drivers/" || true
cp -r "${BUILD_DIR}/root/lib/modules/${KERNEL_VERSION}/kernel/arch/x86/kvm" \
    "${INITRAMFS_ROOT}/lib/modules/${KERNEL_VERSION}/kernel/arch/x86/" || true

# Create init script
cat > "${INITRAMFS_ROOT}/init" << 'EOF'
#!/bin/sh

echo "CognyxOS Host initializing..."

# Mount essential filesystems
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev

# Load essential modules
modprobe kvm_intel || modprobe kvm_amd || true
modprobe vfio_pci || true
modprobe vhost_net || true
modprobe vhost_vsock || true

# Create device nodes
mknod /dev/vfio c 10 196 2>/dev/null || true

# Find and mount root filesystem
ROOT_DEV=$(blkid -L "cognyx-root" || ls /dev/nvme*1 /dev/sda1 2>/dev/null | head -1)

if [ -n "${ROOT_DEV}" ]; then
    echo "Mounting root from ${ROOT_DEV}..."
    mount -o ro "${ROOT_DEV}" /mnt
    
    # Switch to real root
    exec switch_root /mnt /sbin/init
else
    echo "ERROR: Root device not found"
    exec /bin/sh
fi
EOF

chmod +x "${INITRAMFS_ROOT}/init"

# Create initramfs archive
cd "${INITRAMFS_ROOT}"
find . | cpio -H newc -o | gzip -9 > "${BUILD_DIR}/initramfs-cognyx.cpio.gz"
cd "${SCRIPT_DIR}"

# ============================================
# Step 3: Copy configuration files
# ============================================
log_info "Step 3: Copying configuration files..."

cp "${SCRIPT_DIR}/boot/cmdline" "${BUILD_DIR}/"
cp "${SCRIPT_DIR}/boot/grub.cfg" "${BUILD_DIR}/"

# Copy kernel modules
cp "${SCRIPT_DIR}/kernel/scheduler/cognyx_sched.c" "${BUILD_DIR}/" 2>/dev/null || true
cp "${SCRIPT_DIR}/kernel/memory/memguard.c" "${BUILD_DIR}/" 2>/dev/null || true
cp "${SCRIPT_DIR}/kernel/ipc/virtio_ipc.c" "${BUILD_DIR}/" 2>/dev/null || true
cp "${SCRIPT_DIR}/kernel/drivers/gpu/nvidia_vfio.c" "${BUILD_DIR}/" 2>/dev/null || true

# Copy management scripts
cp -r "${SCRIPT_DIR}/storage" "${BUILD_DIR}/"
cp -r "${SCRIPT_DIR}/network" "${BUILD_DIR}/"
cp -r "${SCRIPT_DIR}/virt" "${BUILD_DIR}/"

# ============================================
# Step 4: Create disk image
# ============================================
log_info "Step 4: Creating disk image..."

DISK_IMAGE="${OUTPUT_DIR}/cognyx-host.img"
DISK_SIZE="4G"

# Create sparse file
truncate -s "${DISK_SIZE}" "${DISK_IMAGE}"

# Format as ext4 with label
mkfs.ext4 -L "cognyx-root" -E lazy_itable_init=0,lazy_journal_init=0 \
    "${DISK_IMAGE}"

# Mount and populate
MOUNT_POINT="${BUILD_DIR}/mnt-image"
mkdir -p "${MOUNT_POINT}"
mount -o loop "${DISK_IMAGE}" "${MOUNT_POINT}"

# Create directory structure
mkdir -p "${MOUNT_POINT}"/{boot,etc,var,home,opt,cognyx-root}
mkdir -p "${MOUNT_POINT}/"{bin,sbin,usr/bin,usr/sbin,lib,lib64}
mkdir -p "${MOUNT_POINT}/sys/fs/cgroup"

# Copy kernel and initramfs
cp "${BUILD_DIR}/vmlinuz-cognyx" "${MOUNT_POINT}/boot/"
cp "${BUILD_DIR}/initramfs-cognyx.cpio.gz" "${MOUNT_POINT}/boot/"
cp "${BUILD_DIR}/grub.cfg" "${MOUNT_POINT}/boot/"

# Copy host files
cp -r "${BUILD_DIR}/root/"* "${MOUNT_POINT}/" 2>/dev/null || true
cp -r "${BUILD_DIR}/storage" "${MOUNT_POINT}/opt/cognyx/" 2>/dev/null || true
cp -r "${BUILD_DIR}/network" "${MOUNT_POINT}/opt/cognyx/" 2>/dev/null || true
cp -r "${BUILD_DIR}/virt" "${MOUNT_POINT}/opt/cognyx/" 2>/dev/null || true

# Create /etc/os-release
cat > "${MOUNT_POINT}/etc/os-release" << EOF
NAME="CognyxOS Host"
VERSION="${HOST_VERSION}"
ID=cognyx-host
VERSION_CODENAME=stable
PRETTY_NAME="CognyxOS Host ${HOST_VERSION}"
ANSI_COLOR="1;34"
HOME_URL="https://cognyx.io"
SUPPORT_URL="https://support.cognyx.io"
BUG_REPORT_URL="https://bugs.cognyx.io"
EOF

umount "${MOUNT_POINT}"

# ============================================
# Step 5: Generate checksums
# ============================================
log_info "Step 5: Generating checksums..."

cd "${OUTPUT_DIR}"
sha256sum cognyx-host.img > SHA256SUMS
md5sum cognyx-host.img > MD5SUMS

# ============================================
# Complete
# ============================================
log_info "Build complete!"
log_info "Output: ${OUTPUT_DIR}/cognyx-host.img"
log_info "Size: $(du -h "${OUTPUT_DIR}/cognyx-host.img" | cut -f1)"
log_info "Checksums: ${OUTPUT_DIR}/SHA256SUMS"

echo ""
echo "To deploy:"
echo "  1. Write image to disk: dd if=cognyx-host.img of=/dev/sdX bs=4M status=progress"
echo "  2. Install GRUB on target system"
echo "  3. Boot and verify with: cat /etc/os-release"
