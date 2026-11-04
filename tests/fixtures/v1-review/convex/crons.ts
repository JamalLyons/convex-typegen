import { cronJobs } from "convex/server";
import { internal } from "./_generated/api";

const crons = cronJobs();
crons.daily("fixture cleanup", { hourUTC: 0, minuteUTC: 0 }, internal.admin.users.hiddenMutation);
export default crons;
