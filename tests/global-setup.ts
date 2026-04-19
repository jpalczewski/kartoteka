import fs from "node:fs";
import path from "node:path";

export default async function globalSetup() {
  const dir = path.resolve(__dirname);
  for (const f of ["test.db", "test.db-shm", "test.db-wal"]) {
    const p = path.join(dir, f);
    if (fs.existsSync(p)) fs.rmSync(p);
  }
}
