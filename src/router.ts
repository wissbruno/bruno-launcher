import { createRouter, createWebHashHistory } from 'vue-router';
import HomePage from './pages/HomePage.vue';
import BrowsePage from './pages/BrowsePage.vue';
import ProjectPage from './pages/ProjectPage.vue';
import LibraryPage from './pages/LibraryPage.vue';
import InstancePage from './pages/InstancePage.vue';
import SkinsPage from './pages/SkinsPage.vue';
import SettingsPage from './pages/SettingsPage.vue';

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', name: 'home', component: HomePage },
    { path: '/browse/:type', name: 'browse', component: BrowsePage },
    { path: '/project/:id', name: 'project', component: ProjectPage },
    { path: '/library', name: 'library', component: LibraryPage },
    { path: '/instance/:id', name: 'instance', component: InstancePage },
    { path: '/skins', name: 'skins', component: SkinsPage },
    { path: '/settings', name: 'settings', component: SettingsPage },
  ],
});
