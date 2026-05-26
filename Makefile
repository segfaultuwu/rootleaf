KERNEL := target/x86_64-unknown-none/debug/rootleaf_kernel
ISO := rootleaf.iso

LIMINE_DIR := limine
LIMINE_BRANCH := v11.x-binary

ISO_ROOT := iso_root

DISK_IMG := rootleaf_disk.img
DISK_MB := 64
DISK_DIR := disk_files

USERLAND_DIR := userland
USER_INIT_DIR := $(USERLAND_DIR)/init
USER_TARGET := $(USER_INIT_DIR)/x86_64-rootleaf.json
USER_ELF_BUILT := $(USER_INIT_DIR)/target/x86_64-rootleaf/debug/init
USER_ELF_BIN := $(DISK_DIR)/APP.ELF
USER_ELF_BIN_LOWER := $(DISK_DIR)/app.elf

QEMU := qemu-system-x86_64
CARGO := cargo
XORRISO := xorriso

.PHONY: all kernel limine iso disk userland run run-debug inspect clean distclean check-tools

all: iso disk

check-tools:
	@command -v $(CARGO) >/dev/null || { echo "cargo not found"; exit 1; }
	@command -v $(XORRISO) >/dev/null || { echo "xorriso not found"; exit 1; }
	@command -v git >/dev/null || { echo "git not found"; exit 1; }
	@command -v make >/dev/null || { echo "make not found"; exit 1; }
	@command -v dd >/dev/null || { echo "dd not found"; exit 1; }
	@command -v mcopy >/dev/null || { echo "mcopy not found; install mtools"; exit 1; }
	# need a tool to create ext2/3 images or genext2fs to populate from a directory
	@if ! command -v genext2fs >/dev/null && ! command -v mke2fs >/dev/null; then \
		echo "genext2fs or mke2fs not found; install genext2fs or e2fsprogs"; \
		exit 1; \
	fi
	@command -v $(QEMU) >/dev/null || { echo "qemu-system-x86_64 not found"; exit 1; }

kernel:
	$(CARGO) build

kernel-release:
	$(CARGO) build --release

limine:
	@if [ ! -d "$(LIMINE_DIR)" ]; then \
		git clone https://github.com/limine-bootloader/limine.git \
			--branch=$(LIMINE_BRANCH) \
			--depth=1 \
			$(LIMINE_DIR); \
	fi
	$(MAKE) -C $(LIMINE_DIR) -j$$(nproc)

iso-release: kernel limine
	rm -rf $(ISO_ROOT)
	mkdir -p $(ISO_ROOT)/boot/limine
	mkdir -p $(ISO_ROOT)/EFI/BOOT

	cp cfg/limine.conf $(ISO_ROOT)/boot/limine/limine.conf
	cp target/x86_64-unknown-none/release/rootleaf_kernel $(ISO_ROOT)/boot/kernel.elf

	cp $(LIMINE_DIR)/limine-bios.sys $(ISO_ROOT)/boot/limine/
	cp $(LIMINE_DIR)/limine-bios-cd.bin $(ISO_ROOT)/boot/limine/
	cp $(LIMINE_DIR)/limine-uefi-cd.bin $(ISO_ROOT)/boot/limine/

	cp $(LIMINE_DIR)/BOOTX64.EFI $(ISO_ROOT)/EFI/BOOT/
	cp $(LIMINE_DIR)/BOOTIA32.EFI $(ISO_ROOT)/EFI/BOOT/

	$(XORRISO) -as mkisofs \
		-b boot/limine/limine-bios-cd.bin \
		-no-emul-boot \
		-boot-load-size 4 \
		-boot-info-table \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part \
		--efi-boot-image \
		--protective-msdos-label \
		-partition_offset 16 \
		$(ISO_ROOT) \
		-o $(ISO)

	$(LIMINE_DIR)/limine bios-install $(ISO)

iso: kernel limine
	rm -rf $(ISO_ROOT)
	mkdir -p $(ISO_ROOT)/boot/limine
	mkdir -p $(ISO_ROOT)/EFI/BOOT

	cp cfg/limine.conf $(ISO_ROOT)/boot/limine/limine.conf
	cp $(KERNEL) $(ISO_ROOT)/boot/kernel.elf

	cp $(LIMINE_DIR)/limine-bios.sys $(ISO_ROOT)/boot/limine/
	cp $(LIMINE_DIR)/limine-bios-cd.bin $(ISO_ROOT)/boot/limine/
	cp $(LIMINE_DIR)/limine-uefi-cd.bin $(ISO_ROOT)/boot/limine/

	cp $(LIMINE_DIR)/BOOTX64.EFI $(ISO_ROOT)/EFI/BOOT/
	cp $(LIMINE_DIR)/BOOTIA32.EFI $(ISO_ROOT)/EFI/BOOT/

	$(XORRISO) -as mkisofs \
		-b boot/limine/limine-bios-cd.bin \
		-no-emul-boot \
		-boot-load-size 4 \
		-boot-info-table \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part \
		--efi-boot-image \
		--protective-msdos-label \
		-partition_offset 16 \
		$(ISO_ROOT) \
		-o $(ISO)

	$(LIMINE_DIR)/limine bios-install $(ISO)

$(DISK_DIR):
	mkdir -p $(DISK_DIR)

$(DISK_DIR)/README.TXT: | $(DISK_DIR)
	printf "Rootleaf QEMU disk\n" > $(DISK_DIR)/README.TXT
	printf "This is an ext3 image attached as a second drive.\n" >> $(DISK_DIR)/README.TXT
	printf "Mounted path inside Rootleaf: /disk1\n" >> $(DISK_DIR)/README.TXT

$(DISK_DIR)/NOTES.TXT: | $(DISK_DIR)
	printf "Commands:\n" > $(DISK_DIR)/NOTES.TXT
	printf "  LS /\n" >> $(DISK_DIR)/NOTES.TXT
	printf "  LS /disk1\n" >> $(DISK_DIR)/NOTES.TXT
	printf "  TYPE /disk1/README.TXT\n" >> $(DISK_DIR)/NOTES.TXT
	printf "  ELF /disk1/APP.ELF\n" >> $(DISK_DIR)/NOTES.TXT
	printf "\nLegacy commands may also work:\n" >> $(DISK_DIR)/NOTES.TXT
	printf "  ELF 1:\\APP.ELF\n" >> $(DISK_DIR)/NOTES.TXT
	printf "  TYPE 1:\\README.TXT\n" >> $(DISK_DIR)/NOTES.TXT

userland:
	cd $(USER_INIT_DIR) && \
		cargo +nightly build \
			-Z build-std=core \
			-Z json-target-spec \
			--target x86_64-rootleaf.json

$(USER_ELF_BIN): userland | $(DISK_DIR)
	cp $(USER_ELF_BUILT) $(USER_ELF_BIN)
	cp $(USER_ELF_BUILT) $(USER_ELF_BIN_LOWER)

disk: kernel $(DISK_DIR)/README.TXT $(DISK_DIR)/NOTES.TXT $(USER_ELF_BIN)
	rm -f $(DISK_IMG)
	dd if=/dev/zero of=$(DISK_IMG) bs=1M count=$(DISK_MB) status=none

	# Create ext3 image and populate it. Prefer genext2fs (no root required).
	@if command -v genext2fs >/dev/null; then genext2fs -b $(DISK_MB)M -d $(DISK_DIR) $(DISK_IMG); else mke2fs -q -t ext3 $(DISK_IMG) >/dev/null; if command -v e2cp >/dev/null; then for f in $(DISK_DIR)/*; do e2cp -P -r $$f $(DISK_IMG):/; done; else echo "Created ext3 image but could not populate files; install genext2fs or e2tools (e2cp) to copy files without root"; fi; fi

inspect: disk
	@command -v debugfs >/dev/null && debugfs -R "ls -p /" $(DISK_IMG) || echo "Install debugfs (e2fsprogs) to inspect ext3 images"

run: iso disk
	$(QEMU) \
		-boot order=d \
		-cdrom $(ISO) \
		-drive file=$(DISK_IMG),if=ide,format=raw \
		-m 256M \
		-serial stdio \
		-no-reboot \
		-no-shutdown

run-release: iso-release disk
	$(QEMU) \
		-boot order=d \
		-cdrom $(ISO) \
		-drive file=$(DISK_IMG),if=ide,format=raw \
		-m 256M \
		-serial stdio \
		-no-reboot \
		-no-shutdown

run-debug: iso disk
	$(QEMU) \
		-boot order=d \
		-cdrom $(ISO) \
		-drive file=$(DISK_IMG),if=ide,format=raw \
		-m 256M \
		-serial stdio \
		-no-reboot \
		-no-shutdown \
		-d int,cpu_reset,guest_errors \
		-D qemu.log

clean:
	rm -rf target
	rm -rf $(ISO_ROOT)
	rm -f $(ISO)
	rm -f $(DISK_IMG)
	rm -f $(USER_ELF_BIN)
	rm -f $(USER_ELF_BIN_LOWER)
	rm -rf $(USER_INIT_DIR)/target
	rm -f qemu.log

distclean: clean
	rm -rf $(LIMINE_DIR)
	rm -rf $(DISK_DIR)