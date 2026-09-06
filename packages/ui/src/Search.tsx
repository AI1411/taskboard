import type { Ref } from "react";

import styles from "./Search.module.css";

export function Search(props: {
  value: string;
  onChange: (value: string) => void;
  inputRef?: Ref<HTMLInputElement>;
}) {
  return (
    <input
      ref={props.inputRef}
      className={styles.input}
      type="search"
      aria-label="Search"
      placeholder="Search"
      value={props.value}
      onChange={(e) => props.onChange(e.target.value)}
    />
  );
}
