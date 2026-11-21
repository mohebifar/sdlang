import { requestReset } from "@/lib/auth";
import { trim } from "../../../vendor/strings.js";

export default function Page() {
  return { submit: (email: string) => requestReset(trim(email)) };
}
