FROM lopsided/archlinux:latest

ARG VULKAN_DRIVER=vulkan-broadcom

RUN pacman -Sy --noconfirm vulkan-icd-loader ${VULKAN_DRIVER}

COPY target/release/wosim-headless /usr/local/bin

WORKDIR /world

ENTRYPOINT ["wosim-headless"]

CMD ["serve"]
