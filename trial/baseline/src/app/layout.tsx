// The root layout (unmanaged): every page renders inside it.
import type { ReactNode } from "react";

export const metadata = { title: "Tasks" };

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
