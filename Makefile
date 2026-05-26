KERNEL := target/x86_64-unknown-none/debug/rootleaf_kernel
ISO := rootleaf.iso
LIMINE_DIR := limine
DISK_IMG := rootleaf_disk.img
DISK_MB := 64
DISK_DIR := disk_files

USERLAND_DIR := userland
USER_INIT_DIR := $(USERLAND_DIR)/init
USER_TARGET := $(USER_INIT_DIR)/x86_64-rootleaf.json
USER_ELF_BUILT := $(USER_INIT_DIR)/target/x86_64-rootleaf/debug/init
USER_ELF_BIN := $(DISK_DIR)/APP.ELF

.PHONY: all kernel limine iso disk userland run clean

all: iso disk

kernel:
	cargo build

limine:
	git clone https://github.com/limine-bootloader/limine.git --branch=v11.x-binary --depth=1 $(LIMINE_DIR) || true
	cd $(LIMINE_DIR) && make -j$(nproc)

iso: kernel limine
	mkdir -p iso_root/boot/limine
	cp cfg/limine.conf iso_root/boot/limine/limine.conf
	cp $(KERNEL) iso_root/boot/kernel.elf
	cp $(LIMINE_DIR)/limine-bios.sys iso_root/boot/limine/
	cp $(LIMINE_DIR)/limine-bios-cd.bin iso_root/boot/limine/
	cp $(LIMINE_DIR)/limine-uefi-cd.bin iso_root/boot/limine/
	mkdir -p iso_root/EFI/BOOT
	cp $(LIMINE_DIR)/BOOTX64.EFI iso_root/EFI/BOOT/
	cp $(LIMINE_DIR)/BOOTIA32.EFI iso_root/EFI/BOOT/

	xorriso -as mkisofs \
		-b boot/limine/limine-bios-cd.bin \
		-no-emul-boot \
		-boot-load-size 4 \
		-boot-info-table \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part \
		--efi-boot-image \
		--protective-msdos-label \
		-partition_offset 16 \
		iso_root \
		-o $(ISO)

	$(LIMINE_DIR)/limine bios-install $(ISO)

$(DISK_DIR):
	mkdir -p $(DISK_DIR)

$(DISK_DIR)/README.TXT: | $(DISK_DIR)
	printf "Rootleaf QEMU disk\n" > $(DISK_DIR)/README.TXT
	printf "This is a FAT32 image attached as a second drive.\n" >> $(DISK_DIR)/README.TXT

$(DISK_DIR)/NOTES.TXT: | $(DISK_DIR)
	printf "Commands:\n" > $(DISK_DIR)/NOTES.TXT
	printf "  ELF 1:\\APP.ELF\n" >> $(DISK_DIR)/NOTES.TXT
	printf "  TYPE 1:\\README.TXT\n" >> $(DISK_DIR)/NOTES.TXT

userland:
	cd $(USER_INIT_DIR) && cargo +nightly build -Z build-std=core -Z json-target-spec --target x86_64-rootleaf.json

$(USER_ELF_BIN): userland | $(DISK_DIR)
	cp $(USER_ELF_BUILT) $(USER_ELF_BIN)

disk: kernel $(DISK_DIR)/README.TXT $(DISK_DIR)/NOTES.TXT $(USER_ELF_BIN)
	rm -f $(DISK_IMG)
	dd if=/dev/zero of=$(DISK_IMG) bs=1M count=$(DISK_MB) status=none
	@if command -v mkfs.fat >/dev/null; then \
		mkfs.fat -F 32 $(DISK_IMG) >/dev/null; \
	elif command -v mkfs.vfat >/dev/null; then \
		mkfs.vfat -F 32 $(DISK_IMG) >/dev/null; \
	else \
		echo "mkfs.fat/mkfs.vfat not found"; \
		exit 1; \
	fi
	@if command -v mcopy >/dev/null; then \
		mcopy -i $(DISK_IMG) -s $(DISK_DIR)/* ::/; \
	else \
		echo "mcopy (mtools) not found"; \
		exit 1; \
	fi

run: iso disk
	qemu-system-x86_64 \
		-boot order=d \
		-cdrom $(ISO) \
		-drive file=$(DISK_IMG),if=ide,format=raw \
		-m 256M \
		-serial stdio \
		-no-reboot \
		-no-shutdown

clean:
	rm -rf target iso_root/boot/kernel.elf $(ISO) $(DISK_IMG) $(USER_ELF_BIN)
	rm -rf $(USER_INIT_DIR)/target