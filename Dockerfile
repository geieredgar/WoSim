FROM lopsided/archlinux:latest

ARG VULKAN_DRIVER=vulkan-broadcom

COPY target/release/wosim-headless /usr/local/bin

RUN pacman -Sy --noconfirm vulkan-icd-loader ${VULKAN_DRIVER}

WORKDIR /world

ENV CERT_CHAIN=/cert/fullchain.pem
ENV PRIV_KEY=/cert/privkey.pem

ENTRYPOINT ["wosim-headless"]

CMD ["serve", "--certificate-chain", "${CERT_CHAIN}", "--private-key", "${PRIV_KEY}"]
