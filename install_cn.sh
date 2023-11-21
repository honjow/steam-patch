#!/bin/bash

echo "Installing Steam Patch release..."

OLD_DIR="$HOME/steam-patch"

# 获取 $USER_DIR/steam-patch 的所属用户 如果是root， 则删除
if [ -d "$OLD_DIR" ]; then
    USER_DIR_OWNER=$(stat -c '%U' $OLD_DIR)
    if [ "$USER_DIR_OWNER" == "root" ]; then
        sudo rm -rf $OLD_DIR
    fi
fi

github_prefix=$1
echo "github_prefix: ${github_prefix}"

TEMP_FOLDER=$(mktemp -d)

# Enable CEF debugging
touch "$HOME/.steam/steam/.cef-enable-remote-debugging"

# Download latest release and install it
RELEASE=$(curl -s "${github_prefix}https://api.github.com/repos/honjow/steam-patch/releases" | jq -r "first(.[] | select(.prerelease == "false"))")
VERSION=$(jq -r '.tag_name' <<< ${RELEASE} )
DOWNLOAD_URL=$(jq -r '.assets[].browser_download_url | select(endswith("steam-patch"))' <<< ${RELEASE})

SERVICES_URL=$(jq -r '.assets[].browser_download_url | select(endswith("steam-patch-pro.service"))' <<< ${RELEASE})
SERVICES_BOOT_URL=$(jq -r '.assets[].browser_download_url | select(endswith("restart-steam-patch-on-boot.service"))' <<< ${RELEASE})
CONFIG_URL=$(jq -r '.assets[].browser_download_url | select(endswith("config.toml"))' <<< ${RELEASE})
POLKIT_URL=$(jq -r '.assets[].browser_download_url | select(endswith("steamos-priv-write-updated"))' <<< ${RELEASE})

echo "DOWNLOAD_URL: ${DOWNLOAD_URL}"
echo "SERVICES_URL: ${SERVICES_URL}"
echo "SERVICES_BOOT_URL: ${SERVICES_BOOT_URL}"
echo "CONFIG_URL: ${CONFIG_URL}"
echo "POLKIT_URL: ${POLKIT_URL}"

sudo systemctl --user stop steam-patch 2> /dev/null
sudo systemctl --user disable steam-patch 2> /dev/null

sudo systemctl stop steam-patch 2> /dev/null
sudo systemctl disable steam-patch 2> /dev/null

printf "Installing version %s...\n" "${VERSION}"
curl -L "${github_prefix}${DOWNLOAD_URL}" --output ${TEMP_FOLDER}/steam-patch
curl -L "${github_prefix}${SERVICES_URL}" --output ${TEMP_FOLDER}/steam-patch-pro.service
curl -L "${github_prefix}${SERVICES_BOOT_URL}" --output ${TEMP_FOLDER}/restart-steam-patch-on-boot.service
curl -L "${github_prefix}${CONFIG_URL}" --output ${TEMP_FOLDER}/config.toml
curl -L "${github_prefix}${POLKIT_URL}" --output ${TEMP_FOLDER}/steamos-priv-write-updated

sed -i "s@\$USER@$USER@g" ${TEMP_FOLDER}/steam-patch.service
sudo cp ${TEMP_FOLDER}/steam-patch.service /etc/systemd/system/
sudo cp ${TEMP_FOLDER}/restart-steam-patch-on-boot.service /etc/systemd/system/

polkit_bak_path=/usr/bin/steamos-polkit-helpers/steamos-priv-write.bak
if [ ! -f "$polkit_bak_path" ]; then
    echo "Backing up steamos-priv-write..."
    sudo cp /usr/bin/steamos-polkit-helpers/steamos-priv-write /usr/bin/steamos-polkit-helpers/steamos-priv-write.bak
fi

sudo cp ${TEMP_FOLDER}/steamos-priv-write-updated /usr/bin/steamos-polkit-helpers/steamos-priv-write

chmod +x ${TEMP_FOLDER}/steam-patch
sudo cp ${TEMP_FOLDER}/steam-patch /usr/bin/steam-patch-pro

sudo mkdir -p /etc/steam-patch
config_path=/etc/steam-patch/config.toml
if [ -f "$config_path" ]; then
    echo "Backing up config.toml..."
    cp $config_path "${config_path}.bak"
fi
cp ${TEMP_FOLDER}/config.toml $HOME/steam-patch/config.toml

# DEVICENAME=$(cat /sys/devices/virtual/dmi/id/product_name)
# if [[ "${DEVICENAME}" == "ROG Ally RC71L_RC71L" ]]; then
#     sed -i "s/auto_nkey_recovery = false/auto_nkey_recovery = true/" $HOME/steam-patch/config.toml
# fi

# Run service
sudo systemctl daemon-reload
sudo systemctl enable steam-patch.service
sudo systemctl start steam-patch.service
sudo systemctl enable restart-steam-patch-on-boot.service
sudo systemctl start restart-steam-patch-on-boot.service