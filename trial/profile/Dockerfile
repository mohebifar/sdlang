# Profile trial-v1 (TRIAL.md, Setup): Node.js 22 on Debian bookworm, pinned by the digest
# of its multi-arch index, with pnpm 10 and the Chromium build @playwright/test 1.65.1
# drives. Checks and acceptance runs use it with --network none; only installing a
# workspace's dependencies (tools/make-arm.sh) needs the network.
FROM node:22-bookworm@sha256:e13726ca604184d7a91fcacab1fc09eefd642dc28e79ac34e85540dd93b78cfb

ENV CI=1 \
    PLAYWRIGHT_BROWSERS_PATH=/ms-playwright \
    NEXT_TELEMETRY_DISABLED=1

RUN npm install --global pnpm@10.34.9 \
 && npx --yes playwright@1.65.1 install --with-deps chromium \
 && rm -rf /root/.npm /var/lib/apt/lists/*

WORKDIR /work
