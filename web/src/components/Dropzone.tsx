import { useRef, useState } from "react";
import { useStore } from "../store";

const ACCEPT = "image/png,image/jpeg,image/webp,image/gif,image/bmp";

export function Dropzone() {
  const setSource = useStore((s) => s.setSource);
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);

  function accept(files: FileList | null) {
    const file = files?.[0];
    if (file && file.type.startsWith("image/")) {
      void setSource(file);
    }
  }

  return (
    <div
      className={dragging ? "dropzone dropzone--active" : "dropzone"}
      onDragOver={(e) => {
        e.preventDefault();
        setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragging(false);
        accept(e.dataTransfer.files);
      }}
      onClick={() => inputRef.current?.click()}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          inputRef.current?.click();
        }
      }}
    >
      <p>Drop an image here, or click to choose a file</p>
      <input
        ref={inputRef}
        type="file"
        accept={ACCEPT}
        hidden
        onChange={(e) => accept(e.target.files)}
      />
    </div>
  );
}
