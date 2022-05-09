import api from "../api";
import { isString } from "../typeUtils";
import { BError } from "../error";
import { Persist } from "../persist";
import { authSlice, store } from "../store";
import axios from "axios";

type LoginOpts = { username: string; password: string; site: string };

export const PERSIST_AUTH_KEY = "bulgur-cloud-auth";

function isLoginResponse(data: any): data is api.LoginResponse {
  return isString(data?.token) && Number.isInteger(data?.valid_for_seconds);
}

export class Login {
  async run(data: LoginOpts) {
    const response = await axios.post(
      `/auth/login`,
      {
        username: data.username,
        password: data.password,
      },
      {
        baseURL: data.site,
        headers: {
          "Content-Type": "application/json",
        },
        validateStatus: (status) => status < 500,
      },
    );
    const out = await response?.json();
    if (!isLoginResponse(out)) {
      const status = response?.response.status;
      let reason: string = `${status}`;
      if (status === undefined) reason = `There was an unknown error.`;
      else if (status === 400) {
        reason = "Incorrect username or password";
      } else {
        reason = `There was an unknown error. (${status})`;
      }

      throw new BError({
        code: "login_failed",
        title: "Login failed",
        description: `Unable to log in: ${reason}`,
      });
    }

    const { token } = out;

    console.debug("Persisting the auth token");
    const payload = {
      username: data.username,
      password: data.password,
      token,
      site: data.site,
    };
    await Persist.set(PERSIST_AUTH_KEY, payload);
    store.dispatch(authSlice.actions.login(payload));
    return out;
  }
}
