import { Storage } from "@google-cloud/storage";

const storage = new Storage();

export async function uploadDocument(
  bucketName: string,
  key: string,
  content: Buffer
): Promise<void> {
  const bucket = storage.bucket(bucketName);
  const file = bucket.file(key);
  await file.save(content);
}

export async function downloadDocument(
  bucketName: string,
  key: string
): Promise<Buffer> {
  const bucket = storage.bucket(bucketName);
  const file = bucket.file(key);
  const [contents] = await file.download();
  return contents;
}

export async function listDocuments(
  bucketName: string,
  prefix: string
): Promise<string[]> {
  const bucket = storage.bucket(bucketName);
  const [files] = await bucket.getFiles({ prefix });
  return files.map((file) => file.name);
}
