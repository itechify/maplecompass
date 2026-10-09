/** Mirror of `compass_core::ExpRate`. */
export type ExpRate = { expPerHour: number; percentPerHourHundredths: number | null };

/** Mirror of `compass_core::ExpTracking`. */
export type ExpTracking = {
  state: "off" | "waiting" | "warmingUp" | "active" | "paused";
  timeToLevelS: number | null;
  timeToLevelBadge: string | null;
  rolling: ExpRate | null;
  sessionAverage: ExpRate | null;
  ceilingShareHundredths: number | null;
  deaths: { count: number; expLost: number; percentLostHundredths: number };
  /** The Session started afresh on a character switch and hasn't warmed up since. */
  characterSwitched: boolean;
};
