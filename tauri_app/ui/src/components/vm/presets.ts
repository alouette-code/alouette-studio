import { OsFamily } from "./types";

export const OS_FAMILIES: Record<string, OsFamily> = {
  windows: {
    label: "Windows (x86_64)",
    versions: [
      { id: "win11_24h2", label: "Windows 11 (24H2 / 23H2)", desc: "UEFI + TPM 2.0, 4 Cores, 4GB RAM, 64GB Disk", conf: { cpu: 4, ramMb: 4096, diskGb: 64, fw: "uefi", tpm: true } },
      { id: "win11_lite", label: "Windows 11 Lite / Tiny11", desc: "UEFI + TPM 2.0, 2 Cores, 2GB RAM, 32GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 32, fw: "uefi", tpm: true } },
      { id: "win10", label: "Windows 10 (22H2 / LTSC)", desc: "BIOS, 2 Cores, 4GB RAM, 50GB Disk", conf: { cpu: 2, ramMb: 4096, diskGb: 50, fw: "bios" } },
      { id: "winserver_2025", label: "Windows Server 2025", desc: "UEFI + TPM 2.0, 4 Cores, 8GB RAM, 80GB Disk", conf: { cpu: 4, ramMb: 8192, diskGb: 80, fw: "uefi", tpm: true } },
      { id: "winserver_2022", label: "Windows Server 2022 / 2019", desc: "UEFI, 4 Cores, 8GB RAM, 80GB Disk", conf: { cpu: 4, ramMb: 8192, diskGb: 80, fw: "uefi" } },
      { id: "win8_7", label: "Windows 8.1 / 7 (x64)", desc: "BIOS, 2 Cores, 2GB RAM, 32GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 32, fw: "bios" } },
    ],
  },
  linux: {
    label: "Linux (x86_64)",
    versions: [
      { id: "ubuntu", label: "Ubuntu 24.04 LTS / 24.10 Desktop", desc: "BIOS, 2 Cores, 4GB RAM, 30GB Disk", conf: { cpu: 2, ramMb: 4096, diskGb: 30, fw: "bios" } },
      { id: "ubuntu_server", label: "Ubuntu Server 24.04 LTS", desc: "BIOS, 2 Cores, 2GB RAM, 20GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 20, fw: "bios" } },
      { id: "linux_mint", label: "Linux Mint 22 (Cinnamon)", desc: "BIOS, 2 Cores, 2GB RAM, 25GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 25, fw: "bios" } },
      { id: "pop_os", label: "Pop!_OS 22.04 / 24.04", desc: "UEFI, 2 Cores, 4GB RAM, 30GB Disk", conf: { cpu: 2, ramMb: 4096, diskGb: 30, fw: "uefi" } },
      { id: "zorin_os", label: "Zorin OS 17", desc: "BIOS, 2 Cores, 2GB RAM, 25GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 25, fw: "bios" } },
      { id: "debian", label: "Debian 12 (Bookworm) / 13 (Trixie)", desc: "BIOS, 2 Cores, 2GB RAM, 20GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 20, fw: "bios" } },
      { id: "fedora", label: "Fedora 41 Workstation", desc: "UEFI, 2 Cores, 4GB RAM, 30GB Disk", conf: { cpu: 2, ramMb: 4096, diskGb: 30, fw: "uefi" } },
      { id: "fedora_server", label: "Fedora 41 Server", desc: "UEFI, 2 Cores, 2GB RAM, 25GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 25, fw: "uefi" } },
      { id: "arch", label: "Arch Linux (Rolling)", desc: "UEFI, 2 Cores, 2GB RAM, 25GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 25, fw: "uefi" } },
      { id: "endeavouros", label: "EndeavourOS / CachyOS", desc: "UEFI, 2 Cores, 4GB RAM, 30GB Disk", conf: { cpu: 2, ramMb: 4096, diskGb: 30, fw: "uefi" } },
      { id: "manjaro", label: "Manjaro Linux", desc: "UEFI, 2 Cores, 4GB RAM, 30GB Disk", conf: { cpu: 2, ramMb: 4096, diskGb: 30, fw: "uefi" } },
      { id: "kali", label: "Kali Linux (Rolling)", desc: "BIOS, 2 Cores, 4GB RAM, 40GB Disk", conf: { cpu: 2, ramMb: 4096, diskGb: 40, fw: "bios" } },
      { id: "parrot", label: "Parrot Security OS", desc: "BIOS, 2 Cores, 4GB RAM, 40GB Disk", conf: { cpu: 2, ramMb: 4096, diskGb: 40, fw: "bios" } },
      { id: "rhel", label: "RHEL 9 / Rocky / AlmaLinux 9", desc: "BIOS, 2 Cores, 2GB RAM, 40GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 40, fw: "bios" } },
      { id: "opensuse", label: "openSUSE Tumbleweed / Leap 15.6", desc: "UEFI, 2 Cores, 2GB RAM, 30GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 30, fw: "uefi" } },
      { id: "nixos", label: "NixOS 24.11", desc: "UEFI, 2 Cores, 4GB RAM, 30GB Disk", conf: { cpu: 2, ramMb: 4096, diskGb: 30, fw: "uefi" } },
      { id: "alpine", label: "Alpine Linux 3.21 (Lightweight)", desc: "BIOS, 1 Core, 512MB RAM, 5GB Disk", conf: { cpu: 1, ramMb: 512, diskGb: 5, fw: "bios" } },
      { id: "void_gentoo", label: "Void Linux / Gentoo", desc: "BIOS, 2 Cores, 2GB RAM, 25GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 25, fw: "bios" } },
    ],
  },
  android: {
    label: "Android (x86_64)",
    versions: [
      { id: "bliss_os", label: "Bliss OS 16 (Android 14 x86_64)", desc: "UEFI, 4 Cores, 4GB RAM, 32GB Disk", conf: { cpu: 4, ramMb: 4096, diskGb: 32, fw: "uefi" } },
      { id: "android_11", label: "Android-x86 (11.0 / 9.0)", desc: "BIOS, 2 Cores, 2GB RAM, 16GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 16, fw: "bios" } },
      { id: "prime_os", label: "PrimeOS (x86_64)", desc: "BIOS, 2 Cores, 4GB RAM, 32GB Disk", conf: { cpu: 2, ramMb: 4096, diskGb: 32, fw: "bios" } },
    ],
  },
  bsd_infra: {
    label: "BSD & Infrastructure",
    versions: [
      { id: "freebsd", label: "FreeBSD 14.2", desc: "BIOS, 2 Cores, 2GB RAM, 20GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 20, fw: "bios" } },
      { id: "openbsd", label: "OpenBSD 7.6 / NetBSD 10", desc: "BIOS, 2 Cores, 1GB RAM, 16GB Disk", conf: { cpu: 2, ramMb: 1024, diskGb: 16, fw: "bios" } },
      { id: "proxmox_truenas", label: "Proxmox VE 8 / TrueNAS SCALE", desc: "UEFI, 4 Cores, 8GB RAM, 40GB Disk", conf: { cpu: 4, ramMb: 8192, diskGb: 40, fw: "uefi" } },
      { id: "pfsense_opnsense", label: "pfSense / OPNsense Firewall", desc: "BIOS, 2 Cores, 2GB RAM, 16GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 16, fw: "bios" } },
    ],
  },
  arm64: {
    label: "ARM64 (aarch64)",
    versions: [
      { id: "ubuntu_arm", label: "Ubuntu 24.04 LTS (ARM64)", desc: "UEFI (AAVMF), 2 Cores, 2GB RAM, 25GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 25, fw: "uefi", arch: "aarch64" } },
      { id: "debian_arm", label: "Debian 12 (ARM64)", desc: "UEFI (AAVMF), 2 Cores, 2GB RAM, 20GB Disk", conf: { cpu: 2, ramMb: 2048, diskGb: 20, fw: "uefi", arch: "aarch64" } },
      { id: "alpine_arm", label: "Alpine Linux (ARM64)", desc: "UEFI (AAVMF), 2 Cores, 1GB RAM, 8GB Disk", conf: { cpu: 2, ramMb: 1024, diskGb: 8, fw: "uefi", arch: "aarch64" } },
      { id: "win11_arm", label: "Windows 11 on ARM (ARM64)", desc: "UEFI + TPM 2.0, 4 Cores, 4GB RAM, 64GB Disk", conf: { cpu: 4, ramMb: 4096, diskGb: 64, fw: "uefi", arch: "aarch64", tpm: true } },
    ],
  },
  other: {
    label: "Other / Custom",
    versions: [
      { id: "custom", label: "Custom / Generic x86_64", desc: "BIOS, 1 Core, 1GB RAM, 20GB Disk", conf: { cpu: 1, ramMb: 1024, diskGb: 20, fw: "bios" } },
    ],
  },
};

export function findOsLabel(osType: string | null): string {
  if (!osType) return "Custom / Generic";
  for (const family of Object.values(OS_FAMILIES)) {
    const match = family.versions.find((v) => v.id === osType);
    if (match) return match.label;
  }
  return osType;
}

export function listAllOsOptions(): { id: string; label: string; group: string }[] {
  const out: { id: string; label: string; group: string }[] = [];
  for (const fam of Object.values(OS_FAMILIES)) {
    for (const ver of fam.versions) {
      out.push({ id: ver.id, label: ver.label, group: fam.label });
    }
  }
  return out;
}
