#!/usr/bin/env python3
"""Trigger script invoked by OZ Monitor when a Base Sepolia ResultFulfilled
event fires. Reads the VRF result from Base, asks the OZ Relayer to sign the
proof using the artemis_signer key, then submits submitResult(...) on Artemis
via the artemis_signer relayer.

Stdin: {"monitor_match": ..., "args": [...]}
Required env:
  - RELAYER_API_URL              base URL of OZ Relayer (e.g. http://oz-relayer:8080)
  - RELAYER_API_KEY              bearer token for relayer API
  - BASE_SEPOLIA_RPC_URL         RPC for Base Sepolia (read VRF result)
  - VRF_RESULT_SOURCE_ADDRESS    ChainlinkVRFResultSource address on Base
  - CROSS_CHAIN_RECEIVER_ADDRESS CrossChainResultReceiver address on Artemis

Exits 0 on success.
"""
from __future__ import annotations

import json
import os
import sys
import urllib.request
from typing import Any

from eth_abi import abi as eth_abi
from Crypto.Hash import keccak


SUBMIT_RESULT_SELECTOR = bytes.fromhex("262f08ff")
GET_RESULT_SELECTOR = bytes.fromhex("995e4339")


def fail(msg: str) -> "None":
    print(f"[relay_to_artemis] {msg}", file=sys.stderr)
    sys.exit(1)


def require_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        fail(f"missing required env: {name}")
    return value


def keccak256(data: bytes) -> bytes:
    h = keccak.new(digest_bits=256)
    h.update(data)
    return h.digest()


def http_post_json(url: str, body: dict[str, Any], headers: dict[str, str]) -> dict[str, Any]:
    payload = json.dumps(body).encode()
    req = urllib.request.Request(url, data=payload, method="POST")
    for k, v in headers.items():
        req.add_header(k, v)
    req.add_header("Content-Type", "application/json")
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode())


def rpc_call(rpc_url: str, method: str, params: list[Any]) -> Any:
    body = {"jsonrpc": "2.0", "id": 1, "method": method, "params": params}
    response = http_post_json(rpc_url, body, {})
    if "error" in response:
        fail(f"RPC error: {response['error']}")
    return response["result"]


def find_round_id(payload: dict[str, Any]) -> int:
    """Walk the monitor_match tree and return the first roundId-shaped value."""
    def walk(node: Any) -> int | None:
        if isinstance(node, dict):
            if "roundId" in node:
                return int(str(node["roundId"]), 0)
            for v in node.values():
                result = walk(v)
                if result is not None:
                    return result
        elif isinstance(node, list):
            for v in node:
                result = walk(v)
                if result is not None:
                    return result
        return None

    rid = walk(payload)
    if rid is None:
        fail("could not extract roundId from monitor_match")
    return rid  # type: ignore[return-value]


def main() -> None:
    stdin_data = sys.stdin.read()
    try:
        envelope = json.loads(stdin_data)
    except json.JSONDecodeError as exc:
        fail(f"invalid stdin JSON: {exc}")

    relayer_url = require_env("RELAYER_API_URL").rstrip("/")
    relayer_key = require_env("RELAYER_API_KEY")
    base_rpc_url = require_env("BASE_SEPOLIA_RPC_URL")
    vrf_address = require_env("VRF_RESULT_SOURCE_ADDRESS")
    receiver_address = require_env("CROSS_CHAIN_RECEIVER_ADDRESS")

    round_id = find_round_id(envelope.get("monitor_match", envelope))

    # 1. Read result from Base via eth_call: getResult(uint256)
    get_result_calldata = "0x" + (
        GET_RESULT_SELECTOR + eth_abi.encode(["uint256"], [round_id])
    ).hex()
    raw = rpc_call(
        base_rpc_url,
        "eth_call",
        [{"to": vrf_address, "data": get_result_calldata}, "latest"],
    )
    if not raw or raw == "0x":
        fail(f"empty getResult response for roundId={round_id}")
    (result_tuple,) = eth_abi.decode(["(uint8,uint8[])"], bytes.fromhex(raw[2:]))
    special_prize, all_prizes = result_tuple
    all_prizes = list(all_prizes)

    # 2. Compute the proof hash that the on-chain SingleRelayerVerifier expects:
    #    keccak256(abi.encode(uint256 roundId, uint8 specialPrize, uint8[] allPrizes))
    encoded = eth_abi.encode(
        ["uint256", "uint8", "uint8[]"],
        [round_id, int(special_prize), [int(p) for p in all_prizes]],
    )
    proof_hash = keccak256(encoded)

    # 3. Ask OZ Relayer to sign with artemis_signer.
    #    Relayer's /sign endpoint signs using EIP-191 (personal_sign), which
    #    matches MessageHashUtils.toEthSignedMessageHash on-chain.
    sign_response = http_post_json(
        f"{relayer_url}/api/v1/relayers/artemis_signer/sign",
        {"message": "0x" + proof_hash.hex()},
        {"Authorization": f"Bearer {relayer_key}"},
    )
    signature = sign_response.get("signature")
    if not signature:
        fail(f"relayer /sign returned no signature: {sign_response}")

    # 4. Encode submitResult(uint256, (uint8, uint8[]), bytes) calldata.
    submit_args = eth_abi.encode(
        ["uint256", "(uint8,uint8[])", "bytes"],
        [
            round_id,
            (int(special_prize), [int(p) for p in all_prizes]),
            bytes.fromhex(signature.removeprefix("0x")),
        ],
    )
    submit_data = "0x" + (SUBMIT_RESULT_SELECTOR + submit_args).hex()

    # 5. Submit via artemis_signer relayer.
    tx_response = http_post_json(
        f"{relayer_url}/api/v1/relayers/artemis_signer/transactions",
        {"to": receiver_address, "data": submit_data, "speed": "fast"},
        {"Authorization": f"Bearer {relayer_key}"},
    )
    print(
        f"[relay_to_artemis] roundId={round_id} submitted: "
        f"{json.dumps(tx_response, separators=(',', ':'))}"
    )


if __name__ == "__main__":
    main()
