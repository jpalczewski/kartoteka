-- Better Auth core tables for D1 (SQLite)
-- Generated manually based on Better Auth 1.5.x schema

CREATE TABLE IF NOT EXISTS `user` (
  `id` TEXT NOT NULL PRIMARY KEY,
  `name` TEXT NOT NULL,
  `email` TEXT NOT NULL UNIQUE,
  `emailVerified` INTEGER NOT NULL DEFAULT 0,
  `image` TEXT,
  `createdAt` TEXT NOT NULL DEFAULT (datetime('now')),
  `updatedAt` TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS `session` (
  `id` TEXT NOT NULL PRIMARY KEY,
  `userId` TEXT NOT NULL,
  `token` TEXT NOT NULL UNIQUE,
  `expiresAt` TEXT NOT NULL,
  `ipAddress` TEXT,
  `userAgent` TEXT,
  `createdAt` TEXT NOT NULL DEFAULT (datetime('now')),
  `updatedAt` TEXT NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY (`userId`) REFERENCES `user`(`id`) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS `account` (
  `id` TEXT NOT NULL PRIMARY KEY,
  `userId` TEXT NOT NULL,
  `accountId` TEXT NOT NULL,
  `providerId` TEXT NOT NULL,
  `accessToken` TEXT,
  `refreshToken` TEXT,
  `idToken` TEXT,
  `accessTokenExpiresAt` TEXT,
  `refreshTokenExpiresAt` TEXT,
  `scope` TEXT,
  `password` TEXT,
  `createdAt` TEXT NOT NULL DEFAULT (datetime('now')),
  `updatedAt` TEXT NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY (`userId`) REFERENCES `user`(`id`) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS `verification` (
  `id` TEXT NOT NULL PRIMARY KEY,
  `identifier` TEXT NOT NULL,
  `value` TEXT NOT NULL,
  `expiresAt` TEXT NOT NULL,
  `createdAt` TEXT NOT NULL DEFAULT (datetime('now')),
  `updatedAt` TEXT NOT NULL DEFAULT (datetime('now'))
);
