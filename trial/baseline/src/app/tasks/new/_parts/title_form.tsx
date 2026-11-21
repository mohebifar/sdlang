"use client";

import { useState } from "react";

/** The title field and the Add button, which stays disabled while the title is empty. */
export default function TitleForm({ add }: { add: (formData: FormData) => Promise<void> }) {
  const [title, setTitle] = useState("");
  return (
    <form action={add}>
      <label>
        Title <input name="title" value={title} onChange={(e) => setTitle(e.target.value)} required />
      </label>
      <button type="submit" disabled={title === ""}>
        Add
      </button>
    </form>
  );
}
