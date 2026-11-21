import { signIn } from "../../lib/auth";
import LoginForm from "./_parts/LoginForm";

export default function Page() {
  return LoginForm({ onSubmit: signIn });
}
