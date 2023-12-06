import React, { useState, useEffect } from 'react';
import { Container, Section, Button, Text } from '@app/components';
import { useCustomHook } from '@app/hooks';
import * as app from '@app/core';
export function MyComponent() {
  return null;
}
export default class __Class {
  init() {
    // empty
  }
}
export { app, useCustomHook };
global.__modules.init("test.js");
global.__modules.export("test.js", {
  MyComponent,
  default: __Class,
  app,
  useCustomHook
});
