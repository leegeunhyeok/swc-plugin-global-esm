
import React, { useState, useEffect } from 'react';
import { Container, Section, Button, Text } from '@app/components';
import { useCustomHook } from '@app/hooks';
import * as app from '@app/core';
export function MyComponent() {
  return null;
}
const __export_default = class {
  init() {
    // empty
  }
};
export default __export_default;
export { app, useCustomHook };
global.__modules.export("test.js", {
  MyComponent,
  default: __export_default,
  app,
  useCustomHook
});
