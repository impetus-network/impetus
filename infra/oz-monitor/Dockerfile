FROM openzeppelin/monitor:latest

USER root

# Python deps for relay_to_artemis.py (eth_abi codec + keccak via pycryptodome).
# Base image ships python3 + pip but apk-tools have been pruned, so we add
# packages at the language level only.
RUN python3 -m pip install --break-system-packages --no-cache-dir \
        eth-abi==5.1.0 \
        pycryptodome==3.21.0

# entrypoint renders ${VAR} placeholders in the mounted config template tree
# into /app/config (where the monitor binary looks for its config) at boot.
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh && mkdir -p /app/config && chown nonroot:nonroot /app/config

USER nonroot
ENTRYPOINT ["/entrypoint.sh"]
