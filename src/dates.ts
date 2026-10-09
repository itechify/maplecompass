/** The months as dates are written here: "3 Oct". Not the locale's, which may say "Sept". */
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** A day by the calendar in the player's local time: "3 Oct", or with its year,
 *  "3 Oct 2025". */
export function dayText(date: Date, withYear: boolean) {
  const day = `${date.getDate()} ${MONTHS[date.getMonth()]}`;
  return withYear ? `${day} ${date.getFullYear()}` : day;
}
