/** Mirror of `compass_core::ExpRate`. */
export type ExpRate = { expPerHour: number; percentPerHourHundredths: number | null };

/** Mirror of `compass_core::ExpBar`: the EXP bar's figures, "EXP 160 [3.81%]". */
export type ExpBar = { exp: number; percentHundredths: number };

/** Mirror of `compass_core::VisitBest`: the best one Visit had on a map. */
export type VisitBest = {
  expPerHour: number;
  /** When it was first shown (ms since the Unix epoch), if the wall clock was known. */
  setAtUnixMs: number | null;
};

/** Mirror of `compass_core::PersonalBestLine`: how Last 5 min compares with the best on
 *  the map the player is on. */
export type PersonalBestLine = {
  /** The best the line names: earlier Visits' best, or on a first Visit, this one's. */
  expPerHour: number;
  /** When that best was set (ms since the Unix epoch), if the wall clock was known. */
  setAtUnixMs: number | null;
  /** No earlier Visit had a best here: "Your first best EXP/h here". */
  first: boolean;
  /** Last 5 min's share of the earlier best, once it is 5 minutes of play here. */
  shareHundredths: number | null;
  /** This Visit's best, once it beat the earlier one: the "new best" tag, for the rest of
   *  the Visit. */
  newBest: VisitBest | null;
};

/** Mirror of `compass_core::ExpTracking`. */
export type ExpTracking = {
  state: "off" | "waiting" | "warmingUp" | "active" | "paused" | "unreadable";
  timeToLevelS: number | null;
  timeToLevelBadge: string | null;
  rolling: ExpRate | null;
  sessionAverage: ExpRate | null;
  ceilingShareHundredths: number | null;
  /** The shown map's MeowDB solo EXP/h, while there is a share of it. */
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
  /** The Personal Best line, on a certain map with a Map Summary that has a best to name. */
  personalBest: PersonalBestLine | null;
  /** The player forgot a Personal Best and hasn't left the map since. */
  forgetUndoable: boolean;
};
