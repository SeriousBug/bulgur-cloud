import api from "../api";
import { isBoolean, isString } from "../typeUtils";
import { BError } from "../error";
import { BaseClientCommand } from "./base";
import { joinURL } from "../fetch";
import { AxiosResponse } from "axios";

function isFolderEntry(data: any): data is api.FolderEntry {
  return (
    isBoolean(data?.is_file) &&
    isString(data?.name) &&
    Number.isInteger(data?.size)
  );
}

function isFolderResults(data: any): data is api.FolderResults {
  const entries = data?.entries;
  if (!Array.isArray(entries)) return false;
  return entries.every(isFolderEntry);
}

export const STORAGE = "/storage";

export type FolderResults = api.FolderResults & {
  notFound?: boolean;
};

export class LoadFolder extends BaseClientCommand<FolderResults, [string]> {
  /**
   *
   * @param path The new path to load. Used to navigate to a new folder, or
   * refresh the existing folder.
   *
   * @returns The contents of the requested folder.
   */
  async run(path: string): Promise<FolderResults> {
    const response = await this.get({
      url: joinURL(STORAGE, path),
    });

    console.log("loadFolder", response);
    if (response.status === 404) {
      return {
        entries: [],
        notFound: true,
      };
    }

    const out = await response.data;
    if (!isFolderResults(out)) {
      throw new BError({
        code: "load_folder_failed",
        title: "Failed to load folder",
        description: `Unable to load ${path}`,
        detail: `${response.status} - ${response.statusText}`,
      });
    }

    return out;
  }

  protected async handleError(
    response: AxiosResponse,
  ): Promise<"done" | "continue"> {
    console.log("handleError", response);
    if (response.status === 404) return "done";
    return "continue";
  }
}
