import { useSWRConfig } from "swr";
import api from "../api";
import { BError } from "../error";
import { isBoolean, isString } from "../typeUtils";
import FormData from "form-data";
import { joinURL, urlUp1Level } from "../fetch";
import { isOkResponse, STORAGE } from "./base";
import { RequestParams, useFetch, useRequest } from "./request";
import { storageSlice, useAppDispatch } from "../store";
import { LiveLimit } from "live-limit";

export function usePathExists(url: string) {
  const out = useFetch({
    method: "HEAD",
    url,
  });
  const status = out.data?.status;

  return {
    ...out,
    data: !!(status && isOkResponse(status)),
  };
}

export function usePathToken(url: string) {
  return useFetch<api.StorageAction, api.PathTokenResponse>({
    method: "POST",
    url,
    data: {
      action: "MakePathToken",
    },
  });
}

function useMutateFolder() {
  const { mutate } = useSWRConfig();

  function doMutateFolder(url: string) {
    console.log("Path being mutated", url);
    const mutateParams: RequestParams<never> = {
      method: "GET",
      url,
    };
    mutate(mutateParams);
  }

  function doMutateContainingFolder(url: string) {
    doMutateFolder(urlUp1Level(url.replace(/[/]*$/, "")));
  }

  return { doMutateFolder, doMutateContainingFolder };
}

export function useCreateFolder() {
  const { doRequest } = useRequest<api.StorageAction, never>();
  const { doMutateContainingFolder } = useMutateFolder();

  async function doCreateFolder(url: string) {
    await doRequest({
      url,
      method: "POST",
      data: {
        action: "CreateFolder",
      },
    });
    console.log(url);
    doMutateContainingFolder(url);
  }

  return { doCreateFolder };
}

export function useDelete() {
  const { doRequest } = useRequest<never, never>();
  const { doMutateContainingFolder } = useMutateFolder();

  async function doDelete(url: string) {
    doRequest({
      url,
      method: "DELETE",
    });
    doMutateContainingFolder(url);
  }

  return { doDelete };
}

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

export function useFolderListing(url: string) {
  const resp = useFetch<never, api.FolderResults>({
    url: url,
    method: "GET",
  });

  if (resp.data && isFolderResults(resp.data)) {
    throw new BError({
      code: "load_folder_failed",
      title: "Failed to load folder",
      description: `Unable to load ${url}`,
    });
  }

  return resp;
}

export function useRename() {
  const { doRequest } = useRequest<api.StorageAction, never>();
  const { doMutateContainingFolder } = useMutateFolder();

  async function doRename(from: string, to: string) {
    await doRequest({
      method: "POST",
      url: from,
      data: {
        action: "Move",
        new_path: to,
      },
    });

    doMutateContainingFolder(from);
    const toPath = joinURL(STORAGE, to);
    doMutateContainingFolder(toPath);
  }

  return { doRename };
}

/** Do this many uploads concurrently at maximum. */
const CONCURRENT_UPLOADS = 2;
const UPLOAD_LIMIT = new LiveLimit({ maxLive: CONCURRENT_UPLOADS });

export function useUpload() {
  const { doRequest } = useRequest<FormData, never>();
  const dispatch = useAppDispatch();

  async function doUpload(url: string, files: File[]) {
    files.map((file) => {
      // Mark up all the uploads immediately
      dispatch(
        storageSlice.actions.uploadProgress({
          name: file.name,
          done: 0,
          total: file.size,
        }),
      );
    });

    // The uploads that are in progress right now.
    await Promise.all(
      files.map(async (file) => {
        await UPLOAD_LIMIT.limit(async () => {
          // Perform the request to upload this file
          const name = file.name;
          const form = new FormData();
          form.append(name, file);
          await doRequest(
            {
              method: "PUT",
              url,
              data: form,
            },
            ({ total, done }) => {
              dispatch(
                storageSlice.actions.uploadProgress({
                  name,
                  done,
                  total,
                }),
              );
            },
          );
        });
      }),
    );

    files.map((file) => {
      // Make sure it's removed from the upload progress once the request is done
      dispatch(
        storageSlice.actions.uploadProgress({
          name: file.name,
          done: 0,
          total: 0,
        }),
      );
    });
  }

  return { doUpload };
}
