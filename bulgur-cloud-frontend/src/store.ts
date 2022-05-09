import { configureStore, createSlice } from "@reduxjs/toolkit";
import { TypedUseSelectorHook, useDispatch, useSelector } from "react-redux";
import type { api } from "./api";
import { BError } from "./error";
import { joinURL } from "./fetch";

type LoadState = "done" | "loading" | "uninitialized";

export type AuthState = {
  username?: string;
  password?: string;
  token?: string;
  state: LoadState;
  site?: string;
};

const initialAuthState: AuthState = { state: "uninitialized" };

export type LoginPayload = {
  username: string;
  password: string;
  site: string;
  token: string;
};

export const authSlice = createSlice({
  name: "auth",
  initialState: initialAuthState,
  reducers: {
    setSite: (state, action: { payload: string }) => {
      state.site = action.payload;
    },
    login: (state, action: { payload: LoginPayload }) => {
      state.state = "done";
      state.username = action.payload.username;
      state.password = action.payload.password;
      state.token = action.payload.token;
      state.site = action.payload.site;
    },
    markLoading: (state) => {
      state.state = "loading";
    },
    logout: (state) => {
      state.state = "done";
      state.username = undefined;
      state.password = undefined;
      state.token = undefined;
    },
  },
});

export type StorageState = {
  /** Maps item names to full paths, for items that have been marked to be moved. */
  markedForMove: {
    [fullpath: string]: {
      store: string;
      path: string;
      name: string;
    };
  };
  uploadProgress: {
    [fullpath: string]: {
      size: number;
      uploaded: number;
    };
  };
};

const initialStorageState: StorageState = {
  markedForMove: {},
  uploadProgress: {},
};

export type LoadFolderPayload = api.FolderResults;
export type LoadFilePayload = api.FolderEntry;

export const storageSlice = createSlice({
  name: "storage",
  initialState: initialStorageState,
  reducers: {
    markForMove: (
      state,
      action: { payload: { store: string; path: string; name: string } },
    ) => {
      const { store, path, name } = action.payload;
      const fullPath = joinURL(store, path, name);
      state.markedForMove[fullPath] = {
        store,
        path,
        name,
      };
    },
    unmarkForMove: (
      state,
      action: { payload: { store: string; path: string; name: string } },
    ) => {
      const { store, path, name } = action.payload;
      const fullPath = joinURL(store, path, name);
      if (state.markedForMove[fullPath]) delete state.markedForMove[fullPath];
    },
    clearMarksForMove: (state) => {
      state.markedForMove = {};
    },
    setUploading: (
      state,
      action: {
        payload: { store: string; path: string; name: string; size: number };
      },
    ) => {
      const { store, path, name, size } = action.payload;
      const fullPath = joinURL(store, path, name);
      state.uploadProgress[fullPath] = {
        size,
        uploaded: 0,
      };
    },
    progressUpload: (
      state,
      action: {
        payload: {
          store: string;
          path: string;
          name: string;
          progress: number;
        };
      },
    ) => {
      const { store, path, name, progress } = action.payload;
      const fullPath = joinURL(store, path, name);
      const upload = state.uploadProgress[fullPath];
      upload.uploaded += progress;
      if (upload.uploaded >= upload.size) delete state.uploadProgress[fullPath];
    },
  },
});

type ErrorState = {
  errors: { [key: string]: BError };
};

const initialErrorsState: ErrorState = {
  errors: {},
};

export const errorSlice = createSlice({
  name: "error",
  initialState: initialErrorsState,
  reducers: {
    addError: (state, action: { payload: { key: string; error: unknown } }) => {
      let error: BError;
      if (action.payload.error instanceof BError) {
        error = action.payload.error;
      } else {
        error = new BError({
          code: "unknown_error",
          description: `Please send a bug report with the following message: ${JSON.stringify(
            action.payload.error,
          )}`,
          title: "An unexpected error occured",
        });
      }
      state.errors[action.payload.key] = error;
    },
    clearError: (state, action: { payload: string }) => {
      delete state.errors[action.payload];
    },
  },
});

export const store = configureStore({
  reducer: {
    auth: authSlice.reducer,
    storage: storageSlice.reducer,
    error: errorSlice.reducer,
  },
  middleware: [
    // Log all actions for debugging
    (store: any) => (next: any) => (action: any) => {
      console.group(action?.type);
      console.info("dispatching", action);
      const result = next(action);
      console.log("next state", store.getState());
      console.groupEnd();
      return result;
    },
  ],
});
export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;

// Added to get Typescript to recognize the right type with `typeof ...`
const useAppDispatchWrap = () => useDispatch<AppDispatch>();
export const useAppDispatch: typeof useAppDispatchWrap = useDispatch;

export const useAppSelector: TypedUseSelectorHook<RootState> = useSelector;
