import { ERR_DATA_AND_FORM_DATA } from "./error";

export function normalizeURL(url: string) {
  return url.replace(/\/\/+/g, "/");
}

export function joinURL(...args: string[]) {
  return normalizeURL(args.join("/"));
}

export function urlUp1Level(url: string) {
  const upUrl =
    normalizeURL(url)
      .replace(/\/$/, "")
      .split("/")
      .slice(undefined, -1)
      .join("/") + "/";
  if (upUrl === "/") return "";
  return upUrl;
}

export function urlFileExtension(url: string): string | undefined {
  const extensionMatch = /[.]([^.]+)$/.exec(url);
  const extension = extensionMatch ? extensionMatch[1] : undefined;
  return extension;
}

export function urlFileName(url: string): string | undefined {
  const fileNameMatch = /[/]?([^/]+)$/.exec(url);
  const fileName = fileNameMatch ? fileNameMatch[1] : undefined;
  return fileName;
}

export type Request = {
  site?: string;
  url: string;
  pathToken?: string;
  authToken?: string;
};
export type RequestWithData<Data> = Request & {
  data?: Data;
  formData?: FormData;
};
export type RequestBase<Data> = {
  method: string;
} & Request & { data?: Data; formData?: FormData };
export type ResponseBase =
  | { response: Response; json: () => unknown }
  | undefined;
