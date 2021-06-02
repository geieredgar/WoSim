FROM lopsided/archlinux:latest

ARG VULKAN_DRIVER=vulkan-broadcom

COPY target/release/wosim-headless /usr/local/bin

RUN pacman -Sy --noconfirm vulkan-icd-loader ${VULKAN_DRIVER}

WORKDIR /world

ENTRYPOINT ["wosim-headless"]

CMD ["serve"]
