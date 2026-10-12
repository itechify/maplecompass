/** Mirror of `minerva_core::ExpRate`. */
export type ExpRate = { expPerHour: number; percentPerHourHundredths: number | null };

/** Mirror of `minerva_core::ExpBar`: the EXP bar's figures, "EXP 160 [3.81%]". */
export type ExpBar = { exp: number; percentHundredths: number };

/** Mirror of `minerva_core::ExpTracking`. */
export type ExpTracking = {
  state: "off" | "waiting" | "warmingUp" | "active" | "paused" | "unreadable";
  timeToLevelS: number | null;
  timeToLevelBadge: string | null;
  rolling: ExpRate | null;
  sessionAverage: ExpRate | null;
  ceilingShareHundredths: number | null;
  /** The shown map's solo EXP/h ceiling (its Map Summary), while there is a share of it. */
  soloExpCeiling: number | null;
  deaths: { count: number; expLost: number; percentLostHundredths: number };
  /** The Session started afresh on a character switch and hasn't warmed up since. */
  characterSwitched: boolean;
  /** The Session's active time, and the part of it the Rolling Rate covers (seconds). */
  activeS: number;
  rollingS: number;
  /** The player reset the Session and the EXP hasn't changed since. */
  resetUndoable: boolean;
  /** The last EXP bar reading, which moves with every reading. */
  bar: ExpBar | null;
  /** The Best row: the shown map's Personal Best from its history (EXP/h), from arriving;
   *  none on a first Visit, or where no bests are kept. */
  personalBest: number | null;
  /** The star on the Last 5 min row: the core says when the Rolling Rate beats the Best
   *  row's figure as both are written. */
  newBest: boolean;
};
