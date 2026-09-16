import { useState, type Ref } from "react";

import styles from "./Composer.module.css";

export function Composer(props: {
  placeholder: string;
  onSubmit: (title: string) => void;
  onCancel: () => void;
  inputRef?: Ref<HTMLInputElement>;
}) {
  const [value, setValue] = useState("");

  return (
    <input
      ref={props.inputRef}
      className={styles.input}
      placeholder={props.placeholder}
      value={value}
      autoFocus
      onChange={(e) => setValue(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          const title = value.trim();
          if (title) props.onSubmit(title);
          else props.onCancel();
        }
        if (e.key === "Escape") {
          e.preventDefault();
          props.onCancel();
        }
      }}
    />
  );
}
