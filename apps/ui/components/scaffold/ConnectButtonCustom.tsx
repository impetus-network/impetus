"use client";

import { ConnectButton } from "@rainbow-me/rainbowkit";
import { useDisconnect } from "wagmi";
import { type Address as AddressType, getAddress } from "viem";
import {
  Menu,
  MenuItem,
  MenuPopup,
  MenuSeparator,
  MenuTrigger,
} from "@artemis/coss-ui/ui/menu";
import { Button } from "@artemis/coss-ui/ui/button";
import { BlockieAvatar } from "./BlockieAvatar";
import { Balance } from "./Balance";
import { useCopyToClipboard } from "~/hooks/useCopyToClipboard";
import { useTargetNetwork } from "~/hooks/useTargetNetwork";

function truncateAddress(address: string): string {
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
}

function AddressInfoDropdown({
  address,
  displayName,
}: {
  address: AddressType;
  displayName: string;
}) {
  const { disconnect } = useDisconnect();
  const { copied, copy } = useCopyToClipboard();
  const checkSumAddress = getAddress(address);
  const { targetNetwork } = useTargetNetwork();

  return (
    <Menu>
      <MenuTrigger
        render={
          <Button
            variant="outline"
            size="sm"
            className="h-11 gap-2 rounded-full border-[#e5e5e5] bg-[#f5f0e0] pl-1.5 pr-3 font-semibold text-[#0a0a0a]"
          />
        }
      >
        <BlockieAvatar address={checkSumAddress} size={22} />
        <span className="font-mono text-xs">{truncateAddress(checkSumAddress)}</span>
      </MenuTrigger>
      <MenuPopup>
        <div className="px-3 py-2 border-b border-border">
          <p className="text-sm font-medium">{displayName}</p>
          <p className="font-mono text-xs text-muted-foreground">{truncateAddress(checkSumAddress)}</p>
        </div>
        <MenuItem onClick={() => copy(checkSumAddress)}>
          {copied ? "Copied!" : "Copy Address"}
        </MenuItem>
        <MenuItem
          onClick={() =>
            window.open(`/blockexplorer/address/${checkSumAddress}`, "_blank")
          }
        >
          View on Explorer
        </MenuItem>
        <MenuSeparator />
        <MenuItem className="text-destructive" onClick={() => disconnect()}>
          Disconnect
        </MenuItem>
      </MenuPopup>
    </Menu>
  );
}

function WrongNetworkButton() {
  const { disconnect } = useDisconnect();

  return (
    <Button variant="destructive" size="sm" onClick={() => disconnect()}>
      Wrong Network
    </Button>
  );
}

export function ConnectButtonCustom() {
  const { targetNetwork } = useTargetNetwork();

  return (
    <ConnectButton.Custom>
      {({ account, chain, openConnectModal, mounted }) => {
        const connected = mounted && account && chain;

        if (!connected) {
          return (
            <Button
              size="sm"
              onClick={openConnectModal}
              className="h-11 rounded-xl bg-[#0a0a0a] px-5 font-semibold text-white hover:bg-[#1f1f1f]"
            >
              Connect wallet
            </Button>
          );
        }

        if (chain.unsupported || chain.id !== targetNetwork.id) {
          return <WrongNetworkButton />;
        }

        return (
          <div className="flex items-center gap-2">
            <div className="hidden flex-col items-end sm:flex">
              <span className="font-mono text-xs text-[#0a0a0a]">
                <Balance address={account.address as AddressType} />
              </span>
              <span className="text-[10px] font-medium text-[#6a6a6a]">{chain.name}</span>
            </div>
            <AddressInfoDropdown
              address={account.address as AddressType}
              displayName={account.displayName}
            />
          </div>
        );
      }}
    </ConnectButton.Custom>
  );
}
