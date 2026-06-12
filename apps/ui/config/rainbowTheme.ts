import { type Theme, lightTheme } from "@rainbow-me/rainbowkit";

const base = lightTheme({
  accentColor: "#4f46e5",
  accentColorForeground: "#fafafa",
  borderRadius: "medium",
  fontStack: "system",
  overlayBlur: "small",
});

export const impetusTheme: Theme = {
  ...base,
  colors: {
    ...base.colors,
    accentColor: "#4f46e5",
    accentColorForeground: "#fafafa",
    connectButtonBackground: "#ffffff",
    connectButtonBackgroundError: "#dc2626",
    connectButtonInnerBackground: "#f5f5f5",
    connectButtonText: "#1a1a1a",
    connectButtonTextError: "#ffffff",
    generalBorder: "#e5e5e5",
    generalBorderDim: "#f0f0f0",
    modalBackground: "#ffffff",
    modalBorder: "#e5e5e5",
    modalText: "#1a1a1a",
    modalTextDim: "#737373",
    modalTextSecondary: "#525252",
    menuItemBackground: "#f5f5f5",
    profileAction: "#f5f5f5",
    profileActionHover: "#e5e5e5",
    profileForeground: "#f5f5f5",
    selectedOptionBorder: "#4f46e5",
  },
  fonts: {
    body: "system-ui, -apple-system, sans-serif",
  },
  radii: {
    ...base.radii,
    connectButton: "8px",
    modal: "12px",
    modalMobile: "12px",
  },
  shadows: {
    ...base.shadows,
    connectButton: "0 1px 3px rgba(0, 0, 0, 0.08)",
    dialog: "0 8px 30px rgba(0, 0, 0, 0.12)",
  },
};
