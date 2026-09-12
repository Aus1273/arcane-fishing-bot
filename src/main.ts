import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import DetectionOverlay from './features/calibration/DetectionOverlay.svelte';

const overlay = window.location.hash === '#overlay';
if (overlay) document.documentElement.classList.add('overlay-mode');
export default mount(overlay ? DetectionOverlay : App, { target: document.getElementById('app')! });
