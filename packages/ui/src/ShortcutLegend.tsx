import styles from "./ShortcutLegend.module.css";

const ROWS: { keys: string; action: string }[] = [
  { keys: "j / k", action: "Next / previous card in the current column" },
  { keys: "h / l", action: "Move selected card to the previous / next column" },
  { keys: "Shift+h / Shift+l", action: "Change current column without moving" },
  { keys: "1–4", action: "Jump to column" },
  { keys: "[ / ]", action: "Previous / next project" },
  { keys: "n", action: "New task" },
  { keys: "p", action: "New project" },
  { keys: "u", action: "Toggle urgent" },
  { keys: "e", action: "Focus inspector title" },
  { keys: "/", action: "Search" },
  { keys: "i", action: "Toggle Inbox" },
  { keys: "?", action: "Toggle this shortcut legend" },
  { keys: "Delete / Backspace", action: "Soft-delete selected card" },
  { keys: "Cmd+Z / Ctrl+Z", action: "Undo" },
  { keys: "Esc", action: "Close overlay / clear search / collapse inspector" },
  { keys: "Enter", action: "Open inspector" },
];

export function ShortcutLegend(props: { onClose: () => void }) {
  return (
    <>
      <button type="button" className={styles.backdrop} aria-label="Close shortcuts" onClick={props.onClose} />
      <div className={styles.panel} role="dialog" aria-label="Keyboard shortcuts">
        <div className={styles.header}>
          <h2 className={styles.title}>Keyboard shortcuts</h2>
          <button type="button" className={styles.close} onClick={props.onClose}>
            Close
          </button>
        </div>
        <table className={styles.table}>
          <tbody>
            {ROWS.map((row) => (
              <tr key={row.keys}>
                <th scope="row">{row.keys}</th>
                <td>{row.action}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
