import { create } from "zustand";

interface LayoutState {
  sidebarExpanded: boolean;
  mobileSidebarOpen: boolean;
  toggleSidebar: () => void;
  setSidebarExpanded: (expanded: boolean) => void;
  setMobileSidebarOpen: (open: boolean) => void;
}

export const useLayoutStore = create<LayoutState>((set) => ({
  sidebarExpanded: true,
  mobileSidebarOpen: false,
  toggleSidebar: () =>
    set((state) => ({ sidebarExpanded: !state.sidebarExpanded })),
  setSidebarExpanded: (expanded) => set({ sidebarExpanded: expanded }),
  setMobileSidebarOpen: (open) => set({ mobileSidebarOpen: open }),
}));
