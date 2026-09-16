import styles from "./ConfirmDialog.module.css";

export function ConfirmDialog(props: {
  message: string;
  confirmLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <div className={styles.dialog} role="alertdialog" aria-label={props.message}>
      <p className={styles.message}>{props.message}</p>
      <div className={styles.row}>
        <button type="button" className={styles.confirm} onClick={props.onConfirm}>
          {props.confirmLabel ?? "Delete"}
        </button>
        <button type="button" className={styles.cancel} onClick={props.onCancel}>
          Cancel
        </button>
      </div>
    </div>
  );
}
