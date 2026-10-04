export type Id = string;

export interface User {
  id: Id;
  email: string;
  displayName: string;
  createdAt: string;
}

export interface Session {
  accessToken: string;
  expiresAt: string;
  user: User;
}

export interface EmailMessage {
  id: Id;
  accountId: Id;
  sender: string;
  subject: string;
  bodyText: string;
  folder: string;
  isRead: boolean;
  isStarred: boolean;
  receivedAt: string;
}

export interface SyncCursor {
  accountId: Id;
  cursor: string | null;
  syncedAt: string;
}

export interface ApiError {
  code: string;
  message: string;
}
