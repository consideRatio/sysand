// For loading in browser
let sysand;
beforeAll(async function () {
  if (!sysand) {
    sysand = await import("sysand");
  }
  sysand.init_logger();
  sysand.ensure_debug_hook();
});

beforeEach(async function () {
  sysand.clear_local_storage("sysand_storage/");
});

it("can initialise a project in browser local storage", async function () {
  sysand.init(
    "basic_init",
    "a",
    "1.2.3",
    "sysand_storage",
    "/",
  );
  expect(window.localStorage.getItem("sysand_storage/.project.json")).toBe(
    '{"name":"basic_init","publisher":"a","version":"1.2.3","usage":[]}',
  );
  expect(window.localStorage.getItem("sysand_storage/.meta.json")).toMatch(
    /\{"index":\{\},"created":"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.(\d{3}|\d{6}|\d{9})Z"}/,
  );
});

it("can initialise an empty environment in browser local storage", async function () {
  sysand.env.create("sysand_storage", "/");
  expect(window.localStorage.key(0)).toBe(
    "sysand_storage/sysand_env/entries.txt",
  );
  expect(window.localStorage.key(1)).toBe(null);
  expect(
    window.localStorage.getItem("sysand_storage/sysand_env/entries.txt"),
  ).toBe("");
});
