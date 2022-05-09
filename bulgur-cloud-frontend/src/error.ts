import { Platform } from "react-native";

export type BErrorConstructor = {
  code: string;
  title: string;
  description: string;
  detail?: string;
};

export class BError {
  private _code: string;
  public get code(): string {
    return this._code;
  }
  private _title: string;
  public get title(): string {
    return this._title;
  }
  private _description: string;
  public get description(): string {
    return this._description;
  }
  private _detail?: string;
  public get detail(): string | undefined {
    return this._detail;
  }

  constructor(opts: BErrorConstructor) {
    this._code = opts.code;
    this._title = opts.title;
    this._description = opts.description;
  }
}

export const ERR_NOT_IMPLEMENTED = new BError({
  code: "not_implemented",
  title: "Not implemented",
  description: `This functionality has not been implemented yet for ${Platform.OS}.`,
});

export const ERR_DATA_AND_FORM_DATA = new BError({
  code: "data_and_form_data",
  title: "Internal Implementation Error",
  description:
    "Both the data and the form data fields have been provided for the request.",
});
