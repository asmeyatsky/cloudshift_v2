import {
  S3Client,
  PutObjectCommand,
  GetObjectCommand,
  ListObjectsV2Command,
} from "@aws-sdk/client-s3";

const s3Client = new S3Client({ region: "us-east-1" });

export async function uploadDocument(
  bucketName: string,
  key: string,
  content: Buffer
): Promise<void> {
  const command = new PutObjectCommand({
    Bucket: bucketName,
    Key: key,
    Body: content,
  });
  await s3Client.send(command);
}

export async function downloadDocument(
  bucketName: string,
  key: string
): Promise<Buffer> {
  const command = new GetObjectCommand({
    Bucket: bucketName,
    Key: key,
  });
  const response = await s3Client.send(command);
  const stream = response.Body as NodeJS.ReadableStream;
  const chunks: Buffer[] = [];
  for await (const chunk of stream) {
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks);
}

export async function listDocuments(
  bucketName: string,
  prefix: string
): Promise<string[]> {
  const command = new ListObjectsV2Command({
    Bucket: bucketName,
    Prefix: prefix,
  });
  const response = await s3Client.send(command);
  return (response.Contents ?? []).map((obj) => obj.Key!);
}
