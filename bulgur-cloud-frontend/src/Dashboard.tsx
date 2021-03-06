import {
  Center,
  CloseIcon,
  HStack,
  Icon,
  IconButton,
  Spacer,
  Text,
  VStack,
} from "native-base";
import { File } from "./storage/File";
import { storageSlice, useAppDispatch, useAppSelector } from "./store";
import { FontAwesome5 } from "@expo/vector-icons";
import { urlUp1Level } from "./fetch";
import { FolderList } from "./storage/FolderList";
import { FillSpacer } from "./FillSpacer";
import { DashboardParams, useAppNavigation } from "./routes";
import { useEnsureAuthInitialized, useLogout } from "./client/auth";
import { FullPageLoading } from "./Loading";
import { Banner } from "./components/Banner";

function StorageItem(params: DashboardParams) {
  if (params.route.params.isFile) {
    return <File {...params} />;
  } else {
    return <FolderList {...params} />;
  }
}

function BackButton(params: DashboardParams) {
  const navigation = useAppNavigation();
  const { store, path } = params.route.params;

  if (path === "") {
    // Nothing to back out to
    return (
      <Icon
        as={FontAwesome5}
        color="primary.400"
        name="arrow-left"
        size="md"
        accessibilityElementsHidden={true}
      />
    );
  } else {
    return (
      <Icon
        as={FontAwesome5}
        color="primary.800"
        name="arrow-left"
        size="md"
        accessibilityLabel="Go back"
        onPress={() => {
          navigation.navigate("Dashboard", {
            store,
            path: urlUp1Level(path),
            isFile: false,
          });
        }}
      />
    );
  }
}

export function Dashboard(params: DashboardParams) {
  const username = useAppSelector((selector) => selector.auth.username);
  const { doLogout } = useLogout();
  const state = useEnsureAuthInitialized();

  if (state !== "done") {
    return <FullPageLoading />;
  }

  return (
    <Center>
      <VStack
        width="100%"
        maxWidth="2xl"
        space={4}
        justifyItems="left"
        alignItems="left"
      >
        <Banner bannerKey="page"/>
        <HStack
          space={4}
          marginTop={12}
          marginBottom={4}
          paddingBottom={4}
          borderBottomColor="primary.900"
          borderBottomWidth={2}
        >
          <BackButton {...params} />
          <MiddleSection />
          <HStack space={2}>
            <Text>{username}</Text>
            <Text color="primary.400" onPress={doLogout}>
              (Logout)
            </Text>
          </HStack>
        </HStack>
        <StorageItem {...params} />
      </VStack>
    </Center>
  );
}

function MiddleSection() {
  const dispatch = useAppDispatch();
  const numMarkedForMove = useAppSelector(
    (state) => Object.keys(state.storage.markedForMove).length,
  );

  if (numMarkedForMove === 0) return <FillSpacer />;
  return (
    <FillSpacer>
      <Center>
        <HStack space={2}>
          {numMarkedForMove === 1 ? (
            <Text>{numMarkedForMove} item is marked to be moved</Text>
          ) : (
            <Text>{numMarkedForMove} items are marked to be moved</Text>
          )}
          <Spacer />
          <IconButton
            variant="unstyled"
            accessibilityLabel="Cancel move"
            onPress={() => {
              dispatch(storageSlice.actions.clearMarksForMove());
            }}
            icon={<CloseIcon size="2" color="darkText" />}
          />
        </HStack>
      </Center>
    </FillSpacer>
  );
}
