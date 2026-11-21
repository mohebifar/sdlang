// Sign-in helpers written before adoption.
export async function signIn(email: string, password: string) {
  return email.length > 0 && password.length > 0;
}

export async function requestReset(email: string) {
  return email.includes("@");
}
