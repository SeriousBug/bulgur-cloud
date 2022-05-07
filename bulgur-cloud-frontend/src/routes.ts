import {
  getStateFromPath,
  getPathFromState,
  Route,
} from "@react-navigation/native";
import {
  createNativeStackNavigator,
  NativeStackScreenProps,
} from "@react-navigation/native-stack";

export type RoutingStackParams = {
  Login: undefined;
  Dashboard: {
    store: string;
    path: string;
    isFile: boolean;
  };
  NotFound: undefined;
};

export const Stack: any = createNativeStackNavigator<RoutingStackParams>();
export const LINKING = {
  prefixes: ["bulgur-cloud://"],
  config: {
    screens: {
      Login: "",
      Dashboard: "s/:store/",
      NotFound: "*",
    },
  },
  getStateFromPath: (path: string, config: any) => {
    if (path.startsWith("/s/")) {
      const matches =
        /^[/]s[/](?<store>[^/]+)[/](?<path>.*?)(?<trailingSlash>[/]?)$/.exec(
          path,
        );
      const out = {
        routes: [
          {
            name: "Dashboard",
            path,
            params: {
              store: matches?.groups?.store,
              path: matches?.groups?.path,
              isFile:
                matches?.groups?.trailingSlash === "" &&
                matches?.groups?.path !== "",
            },
          },
        ],
      };
      return out;
    }
    const state = getStateFromPath(path, config);
    return state;
  },
  getPathFromState: (state: any, config: any) => {
    const route = state.routes[0];
    if (route?.name === "Dashboard") {
      const params: RoutingStackParams["Dashboard"] = route.params;
      let path = params.path;
      // If it's a folder, make sure it has a trailing slash
      if (!params.isFile && !path.endsWith("/") && path !== "") {
        path = `${path}/`;
      }
      return `/s/${params.store}/${path}`;
    }
    return getPathFromState(state, config);
  },
};

export type DashboardParams = NativeStackScreenProps<
  RoutingStackParams,
  "Dashboard"
>;
export type NotFoundParams = NativeStackScreenProps<
  RoutingStackParams,
  "NotFound"
>;
export type DashboardRoute = Route<
  "Dashboard",
  RoutingStackParams["Dashboard"]
>;

export function isDashboardRoute(
  route?: Route<string, any>,
): route is DashboardRoute {
  return route?.name === "Dashboard";
}

declare global {
  namespace ReactNavigation {
    interface RootParamList extends RoutingStackParams {}
  }
}
