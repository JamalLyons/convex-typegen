import { httpRouter } from "convex/server";
import { httpAction } from "./_generated/server";

const http = httpRouter();
http.route({ path: "/ping", method: "GET", handler: httpAction(async () => new Response("pong")) });
export default http;
