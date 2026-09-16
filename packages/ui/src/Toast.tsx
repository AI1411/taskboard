import { useEffect } from "react";

import styles from "./Toast.module.css";

export function Toast(props: {
  message: string;
  error?: boolean;
  action?: { label: string; onClick: () => void };
  onDismiss: () => void;
}) {
  useEffect(() => {
    const ms = props.error ? 8000 : 2400;
    const id = window.setTimeout(() => props.onDismiss(), ms);
    return () => window.clearTimeout(id);
  }, [props.message, props.error, props.onDismiss]);

  return (
    <div
      className={`${styles.toast} ${props.error ? styles.error : ""}`}
      role="status"
      aria-live="polite"
    >
      <span>{props.message}</span>
      {props.action ? (
        <button type="button" className={styles.action} onClick={props.action.onClick}>
          {props.action.label}
        </button>
      ) : null}
      {props.error ? (
        <button type="button" className={styles.dismiss} aria-label="Dismiss" onClick={props.onDismiss}>
          Dismiss
        </button>
      ) : null}
    </div>
  );
}
