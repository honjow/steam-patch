#!/bin/bash

echo "Installing Steam Patch release..."

CURRENT_WD=$(pwd)

# Enable CEF debugging
touch "$HOME/.steam/steam/.cef-enable-remote-debugging"

which dnf 2>/dev/null
FEDORA_BASE=$?

cat /etc/nobara-release
NOBARA=$?

if [ $FEDORA_BASE == 0 ]; then
	echo -e '\nFedora based installation starting.\n'
	sudo dnf install cargo
	mkdir -p $HOME/rpmbuild/{SPECS,SOURCES}
	cp steam-patch.spec $HOME/rpmbuild/SPECS
	rpmbuild -bb $HOME/rpmbuild/SPECS/steam-patch.spec
	sudo dnf install $HOME/rpmbuild/RPMS/x86_64/steam-patch*.rpm
fi

which pacman 2>/dev/null
ARCH_BASE=$?

cat /etc/os-release | grep ChimeraOS
CHIMERA_BASE=$?

if [ $ARCH_BASE == 0 ]; then
	echo -e '\nArch based installation starting.\n'
	if [ $CHIMERA_BASE == 0 ]; then
        	sudo frzr-unlock
	fi
	sudo pacman -Sy --noconfirm cargo gcc
	printf "Installing steam-patch...\n"
	cargo build -r
	chmod +x $CURRENT_WD/target/release/steam-patch
	sudo cp $CURRENT_WD/target/release/steam-patch /usr/bin/steam-patch
	sed -i "s@\$USER@$USER@g" steam-patch.service
	sudo cp steam-patch.service /etc/systemd/system/
	sudo cp restart-steam-patch-on-boot.service /etc/systemd/system/
	sudo cp /usr/bin/steamos-polkit-helpers/steamos-priv-write /usr/bin/steamos-polkit-helpers/steamos-priv-write-bkp
	sudo cp steamos-priv-write-updated /usr/bin/steamos-polkit-helpers/steamos-priv-write
	# Run service
	systemctl daemon-reload
	systemctl enable steam-patch.service
	systemctl start steam-patch.service
	systemctl enable restart-steam-patch-on-boot.service
	systemctl start restart-steam-patch-on-boot.service
fi
