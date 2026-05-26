KERNEL := target/x86_64-unknown-none/debug/rootleaf_kernel
ISO := rootleaf.iso
LIMINE_DIR := limine

.PHONY: all kernel limine iso run clean

all: iso

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

run: iso
	qemu-system-x86_64 \
		-cdrom $(ISO) \
		-m 256M \
		-serial stdio \
		-no-reboot \
		-no-shutdown

clean:
	rm -rf target iso_root/boot/kernel.elf $(ISO)