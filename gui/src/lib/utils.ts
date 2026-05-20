import { useEffect, useRef } from 'react';
import { useLocation } from 'react-router-dom';
import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/** Re-run `fn` whenever the current route matches `path`. */
export function usePageActive(path: string, fn: () => void) {
  const location = useLocation();
  const fnRef = useRef(fn);
  fnRef.current = fn;
  useEffect(() => {
    if (location.pathname === path) fnRef.current();
  }, [location.pathname, path]);
}
